//! 条带图渲染器
//!
//! 将 ChartSpec 和 DataTable 渲染为条带图的 Canvas 指令。
//! 使用 beeswarm 算法避免点重叠。

use crate::layout::{compute_layout, PlotArea};
use crate::spec::ChartSpec;
use crate::theme::Theme;
use crate::chart::ChartOutput;
use crate::error::ComponentError;
use deneb_core::*;
use deneb_core::algorithm::beeswarm::{beeswarm_layout, StripLayout};
use std::collections::HashMap;

/// StripChart 渲染器
pub struct StripChart;

impl StripChart {
    /// 渲染条带图
    pub fn render<T: Theme>(
        spec: &ChartSpec,
        theme: &T,
        data: &DataTable,
    ) -> Result<ChartOutput, ComponentError> {
        // 1. 验证数据
        Self::validate_data(spec, data)?;

        // 空数据检查
        if data.is_empty() || data.row_count() == 0 {
            return Ok(Self::render_empty(spec, theme));
        }

        // 2. 计算布局
        let layout = compute_layout(spec, theme, data);
        let plot_area = &layout.plot_area;

        // 3. 构建 Scale
        let (x_scale, y_scale) = Self::build_scales(spec, data, plot_area)?;

        // 4. 按类别分组
        let groups = Self::group_by_category(spec, data)?;

        // 5. 生成渲染指令
        let mut layers = RenderLayers::new();
        let mut hit_regions = Vec::new();

        // 背景层
        layers.update_layer(LayerKind::Background, super::shared::render_background_with_border(spec, theme, plot_area));

        // 网格层
        if let Some(y_axis) = &layout.y_axis {
            layers.update_layer(LayerKind::Grid, super::shared::render_grid_horizontal(theme, &y_axis.tick_positions, plot_area));
        }

        // 数据层（散点）
        let point_radius = theme.layout_config().point_radius;
        let (data_commands, point_regions) = Self::render_points(
            theme,
            &x_scale,
            &y_scale,
            &groups,
            point_radius,
        )?;
        layers.update_layer(LayerKind::Data, data_commands);
        hit_regions.extend(point_regions);

        // 轴层
        layers.update_layer(LayerKind::Axis, super::shared::render_axes(spec, theme, &layout, plot_area, true));

        // 标题层
        if let Some(title) = &spec.title {
            layers.update_layer(LayerKind::Title, super::shared::render_title(theme, title, plot_area));
        }

        Ok(ChartOutput {
            layers,
            hit_regions,
        })
    }

    /// 验证数据
    fn validate_data(spec: &ChartSpec, data: &DataTable) -> Result<(), ComponentError> {
        if let Some(x_field) = &spec.encoding.x {
            if data.get_column(&x_field.name).is_none() {
                return Err(ComponentError::InvalidConfig {
                    reason: format!("x field '{}' not found in data", x_field.name),
                });
            }
        }

        if let Some(y_field) = &spec.encoding.y {
            if data.get_column(&y_field.name).is_none() {
                return Err(ComponentError::InvalidConfig {
                    reason: format!("y field '{}' not found in data", y_field.name),
                });
            }
        }

        Ok(())
    }

    /// 渲染空数据
    fn render_empty<T: Theme>(spec: &ChartSpec, theme: &T) -> ChartOutput {
        let mut layers = RenderLayers::new();
        let plot_area = PlotArea {
            x: theme.margin().left,
            y: theme.margin().top,
            width: spec.width - theme.margin().horizontal(),
            height: spec.height - theme.margin().vertical(),
        };

        layers.update_layer(LayerKind::Background, super::shared::render_background_with_border(spec, theme, &plot_area));

        if let Some(title) = &spec.title {
            layers.update_layer(LayerKind::Title, super::shared::render_title(theme, title, &plot_area));
        }

        ChartOutput {
            layers,
            hit_regions: Vec::new(),
        }
    }

    /// 构建比例尺
    fn build_scales(
        spec: &ChartSpec,
        data: &DataTable,
        plot_area: &PlotArea,
    ) -> Result<(BandScale, LinearScale), ComponentError> {
        let x_field = spec.encoding.x.as_ref().ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: "x encoding is required".to_string(),
            }
        })?;

        let x_column = data.get_column(&x_field.name).ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: format!("x field '{}' not found", x_field.name),
            }
        })?;

        let categories: Vec<String> = x_column
            .values
            .iter()
            .filter_map(|v| v.as_text().map(|s| s.to_string()))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let x_scale = BandScale::new(
            categories,
            plot_area.x,
            plot_area.x + plot_area.width,
            0.1,
        );

        let y_field = spec.encoding.y.as_ref().ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: "y encoding is required".to_string(),
            }
        })?;

        let y_column = data.get_column(&y_field.name).ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: format!("y field '{}' not found", y_field.name),
            }
        })?;

        let mut min: Option<f64> = None;
        let mut max: Option<f64> = None;

        for value in &y_column.values {
            if let Some(num) = value.as_numeric() {
                min = Some(min.map_or(num, |m| m.min(num)));
                max = Some(max.map_or(num, |m| m.max(num)));
            }
        }

        let (min, max) = match (min, max) {
            (Some(min), Some(max)) => (min, max),
            _ => (0.0, 100.0),
        };

        // Strip chart Y 轴不从 0 开始（位置编码）
        let y_scale = LinearScale::new(
            min,
            max,
            plot_area.y + plot_area.height,
            plot_area.y,
        );

        Ok((x_scale, y_scale))
    }

    /// 按类别分组数据
    fn group_by_category(
        spec: &ChartSpec,
        data: &DataTable,
    ) -> Result<HashMap<String, Vec<(f64, usize, Vec<FieldValue>)>>, ComponentError> {
        let x_field = spec.encoding.x.as_ref().ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: "x encoding is required".to_string(),
            }
        })?;

        let y_field = spec.encoding.y.as_ref().ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: "y encoding is required".to_string(),
            }
        })?;

        let x_column = data.get_column(&x_field.name).ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: format!("x field '{}' not found", x_field.name),
            }
        })?;

        let y_column = data.get_column(&y_field.name).ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: format!("y field '{}' not found", y_field.name),
            }
        })?;

        let mut groups: HashMap<String, Vec<(f64, usize, Vec<FieldValue>)>> = HashMap::new();
        let row_count = data.row_count();

        for row_idx in 0..row_count {
            let x_value = x_column
                .get(row_idx)
                .and_then(|v| v.as_text())
                .unwrap_or("")
                .to_string();

            let y_value = y_column
                .get(row_idx)
                .and_then(|v| v.as_numeric())
                .unwrap_or(0.0);

            // 收集该行的所有字段值
            let mut row_data = Vec::new();
            for column in &data.columns {
                if let Some(value) = column.get(row_idx) {
                    row_data.push(value.clone());
                }
            }

            groups
                .entry(x_value)
                .or_insert_with(Vec::new)
                .push((y_value, row_idx, row_data));
        }

        Ok(groups)
    }

    /// 渲染散点（带 beeswarm 布局）
    fn render_points<T: Theme>(
        theme: &T,
        x_scale: &BandScale,
        y_scale: &LinearScale,
        groups: &HashMap<String, Vec<(f64, usize, Vec<FieldValue>)>>,
        point_radius: f64,
    ) -> Result<(RenderOutput, Vec<HitRegion>), ComponentError> {
        let mut output = RenderOutput::new();
        let mut hit_regions = Vec::new();

        let mut group_idx = 0;
        for (category, points) in groups {
            let band_center = x_scale.band_center(category).ok_or_else(|| {
                ComponentError::InvalidConfig {
                    reason: format!("category not found in x scale: {}", category),
                }
            })?;

            let band_width = x_scale.band_width();

            // 提取 y 值用于 beeswarm 计算
            let y_values: Vec<f64> = points.iter().map(|(v, _, _)| *v).collect();

            // 计算 x 偏移量
            let offsets = beeswarm_layout(
                &y_values,
                StripLayout::Beeswarm,
                point_radius,
                band_width,
            );

            // 确定颜色
            let color = if groups.len() > 1 {
                theme.series_color(group_idx).to_string()
            } else {
                theme.series_color(0).to_string()
            };

            for (i, (y_val, row_idx, row_data)) in points.iter().enumerate() {
                let cx = band_center + offsets[i];
                let cy = y_scale.map(*y_val);

                output.add_command(DrawCmd::Circle {
                    cx,
                    cy,
                    r: point_radius,
                    fill: Some(FillStyle::Color(color.clone())),
                    stroke: None,
                });

                let region = HitRegion::from_point(
                    cx,
                    cy,
                    point_radius,
                    *row_idx,
                    if groups.len() > 1 { Some(group_idx) } else { None },
                    row_data.clone(),
                );
                hit_regions.push(region);
            }

            group_idx += 1;
        }

        Ok((output, hit_regions))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{Encoding, Mark, Field};
    use crate::theme::DefaultTheme;

    fn create_test_data() -> DataTable {
        DataTable::with_columns(vec![
            Column::new(
                "category",
                DataType::Nominal,
                vec![
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("B".to_string()),
                    FieldValue::Text("B".to_string()),
                    FieldValue::Text("B".to_string()),
                ],
            ),
            Column::new(
                "value",
                DataType::Quantitative,
                vec![
                    FieldValue::Numeric(10.0),
                    FieldValue::Numeric(15.0),
                    FieldValue::Numeric(20.0),
                    FieldValue::Numeric(30.0),
                    FieldValue::Numeric(35.0),
                    FieldValue::Numeric(40.0),
                ],
            ),
        ])
    }

    fn create_test_spec() -> ChartSpec {
        ChartSpec::builder()
            .mark(Mark::Strip)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("category"))
                    .y(Field::quantitative("value")),
            )
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap()
    }

    #[test]
    fn test_strip_chart_render_basic() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = create_test_data();

        let result = StripChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 6); // 6 个数据点
        assert!(output.layers.get_layer(LayerKind::Background).is_some());
        assert!(output.layers.get_layer(LayerKind::Data).is_some());
    }

    #[test]
    fn test_strip_chart_render_empty_data() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("category", DataType::Nominal, vec![]),
            Column::new("value", DataType::Quantitative, vec![]),
        ]);

        let result = StripChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.hit_regions.is_empty());
        assert!(output.layers.get_layer(LayerKind::Background).is_some());
    }

    #[test]
    fn test_strip_chart_validate_missing_field() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("wrong", DataType::Nominal, vec![]),
        ]);

        let result = StripChart::render(&spec, &theme, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_strip_chart_single_point() {
        let data = DataTable::with_columns(vec![
            Column::new(
                "category",
                DataType::Nominal,
                vec![FieldValue::Text("A".to_string())],
            ),
            Column::new(
                "value",
                DataType::Quantitative,
                vec![FieldValue::Numeric(42.0)],
            ),
        ]);

        let spec = create_test_spec();
        let theme = DefaultTheme;

        let result = StripChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 1);
    }

    #[test]
    fn test_strip_chart_with_title() {
        let spec = ChartSpec::builder()
            .mark(Mark::Strip)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("category"))
                    .y(Field::quantitative("value")),
            )
            .title("Strip Chart Test")
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap();

        let theme = DefaultTheme;
        let data = create_test_data();

        let result = StripChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        let title_layer = output.layers.get_layer(LayerKind::Title);
        assert!(title_layer.is_some());
        assert!(!title_layer.unwrap().commands.is_empty());
    }

    #[test]
    fn test_strip_chart_layers() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = create_test_data();

        let result = StripChart::render(&spec, &theme, &data).unwrap();

        assert!(result.layers.get_layer(LayerKind::Background).is_some());
        assert!(result.layers.get_layer(LayerKind::Grid).is_some());
        assert!(result.layers.get_layer(LayerKind::Axis).is_some());
        assert!(result.layers.get_layer(LayerKind::Data).is_some());
    }

    #[test]
    fn test_strip_chart_single_category() {
        let data = DataTable::with_columns(vec![
            Column::new(
                "category",
                DataType::Nominal,
                vec![
                    FieldValue::Text("X".to_string()),
                    FieldValue::Text("X".to_string()),
                    FieldValue::Text("X".to_string()),
                    FieldValue::Text("X".to_string()),
                    FieldValue::Text("X".to_string()),
                ],
            ),
            Column::new(
                "value",
                DataType::Quantitative,
                vec![
                    FieldValue::Numeric(10.0),
                    FieldValue::Numeric(20.0),
                    FieldValue::Numeric(30.0),
                    FieldValue::Numeric(40.0),
                    FieldValue::Numeric(50.0),
                ],
            ),
        ]);

        let spec = create_test_spec();
        let theme = DefaultTheme;

        let result = StripChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 5);
    }
}
