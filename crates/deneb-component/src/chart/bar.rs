//! 柱状图渲染器
//!
//! 将 ChartSpec 和 DataTable 渲染为柱状图的 Canvas 指令。

use crate::layout::{compute_layout, PlotArea};
use crate::spec::ChartSpec;
use crate::theme::Theme;
use crate::chart::ChartOutput;
use crate::error::ComponentError;
use deneb_core::*;
use std::collections::HashMap;

/// BarChart 渲染器
pub struct BarChart;

impl BarChart {
    /// 渲染柱状图
    ///
    /// # Arguments
    ///
    /// * `spec` - 图表规格
    /// * `theme` - 主题
    /// * `data` - 数据表
    ///
    /// # Returns
    ///
    /// 返回渲染结果和命中区域
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

        // 4. 按系列分组（如果有 color encoding）
        let series_data = Self::group_by_series(spec, data)?;

        // 5. 生成渲染指令
        let mut layers = RenderLayers::new();
        let mut hit_regions = Vec::new();

        // 背景层
        layers.update_layer(LayerKind::Background, super::shared::render_background_with_border(spec, theme, plot_area));

        // 网格层
        if let Some(y_axis) = &layout.y_axis {
            layers.update_layer(LayerKind::Grid, super::shared::render_grid_horizontal(theme, &y_axis.tick_positions, plot_area));
        }

        // 数据层（柱子）
        let (data_commands, bar_regions) = Self::render_bars(
            spec,
            theme,
            &x_scale,
            &y_scale,
            &series_data,
            plot_area,
        )?;
        layers.update_layer(LayerKind::Data, data_commands);
        hit_regions.extend(bar_regions);

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
        // 检查必需的字段是否存在
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

        if let Some(color_field) = &spec.encoding.color {
            if data.get_column(&color_field.name).is_none() {
                return Err(ComponentError::InvalidConfig {
                    reason: format!("color field '{}' not found in data", color_field.name),
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

        // 背景层
        layers.update_layer(LayerKind::Background, super::shared::render_background_with_border(spec, theme, &plot_area));

        // 标题层
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
        // X 轴：BandScale（类别列）
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

        // 获取唯一类别
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

        // Y 轴：LinearScale（数值列）
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

        // 获取数值范围
        let mut min: Option<f64> = None;
        let mut max: Option<f64> = None;

        for value in &y_column.values {
            if let Some(num) = value.as_numeric() {
                min = Some(min.map_or(num, |m| m.min(num)));
                max = Some(max.map_or(num, |m| m.max(num)));
            }
        }

        // 如果全是空值，使用默认范围
        let (min, max) = match (min, max) {
            (Some(min), Some(max)) => (min, max),
            _ => (0.0, 100.0),
        };

        // 确保 0 在范围内（处理负值）
        let min = min.min(0.0);
        let max = max.max(0.0);

        // Y 轴 range 是反向的（底部=max_y，顶部=min_y）
        let y_scale = LinearScale::new(
            min,
            max,
            plot_area.y + plot_area.height,
            plot_area.y,
        );

        Ok((x_scale, y_scale))
    }

    /// 按系列分组数据
    fn group_by_series(
        spec: &ChartSpec,
        data: &DataTable,
    ) -> Result<HashMap<Option<String>, Vec<(String, f64, usize, Vec<FieldValue>)>>, ComponentError> {
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

        let color_column = spec.encoding.color.as_ref()
            .and_then(|field| data.get_column(&field.name));

        let mut series_data: HashMap<Option<String>, Vec<(String, f64, usize, Vec<FieldValue>)>> = HashMap::new();

        let row_count = data.row_count();
        for row_idx in 0..row_count {
            // 获取 x 值
            let x_value = x_column
                .get(row_idx)
                .and_then(|v| v.as_text())
                .unwrap_or("")
                .to_string();

            // 获取 y 值
            let y_value = y_column
                .get(row_idx)
                .and_then(|v| v.as_numeric())
                .unwrap_or(0.0);

            // 获取系列值
            let series = color_column
                .and_then(|col| col.get(row_idx))
                .and_then(|v| v.as_text())
                .map(|s| s.to_string());

            // 收集该行的所有字段值
            let mut row_data = Vec::new();
            for column in &data.columns {
                if let Some(value) = column.get(row_idx) {
                    row_data.push(value.clone());
                }
            }

            series_data
                .entry(series)
                .or_insert_with(Vec::new)
                .push((x_value, y_value, row_idx, row_data));
        }

        Ok(series_data)
    }

    /// 渲染柱子
    fn render_bars<T: Theme>(
        _spec: &ChartSpec,
        theme: &T,
        x_scale: &BandScale,
        y_scale: &LinearScale,
        series_data: &HashMap<Option<String>, Vec<(String, f64, usize, Vec<FieldValue>)>>,
        _plot_area: &PlotArea,
    ) -> Result<(RenderOutput, Vec<HitRegion>), ComponentError> {
        let mut output = RenderOutput::new();
        let mut hit_regions = Vec::new();

        let series_keys: Vec<_> = series_data.keys().cloned().collect();
        let series_count = series_keys.len();

        // 计算基线位置（y=0 对应的像素位置）
        let baseline = y_scale.map(0.0);

        for (series_idx, series_key) in series_keys.iter().enumerate() {
            let bars = series_data.get(series_key).ok_or_else(|| {
                ComponentError::InvalidConfig {
                    reason: format!("series data not found: {:?}", series_key),
                }
            })?;

            for (bar_idx, (category, value, row_idx, row_data)) in bars.iter().enumerate() {
                // 多系列按系列分色，单系列按类别分色
                let color = if series_count > 1 {
                    theme.series_color(series_idx).to_string()
                } else {
                    theme.series_color(bar_idx).to_string()
                };
                // 计算柱子位置
                let band_start = x_scale.band_start(category).ok_or_else(|| {
                    ComponentError::InvalidConfig {
                        reason: format!("category not found in x scale: {}", category),
                    }
                })?;

                let band_width = x_scale.band_width();

                // 多系列时，将每个 band 细分
                let (bar_x, bar_width) = if series_count > 1 {
                    let sub_width = band_width / series_count as f64;
                    let offset = series_idx as f64 * sub_width;
                    (band_start + offset, sub_width)
                } else {
                    (band_start, band_width)
                };

                // 计算柱子高度和位置
                let y_pos = y_scale.map(*value);

                // 处理正负值
                let (bar_y, bar_height) = if *value >= 0.0 {
                    // 正值：从基线向上
                    let height = baseline - y_pos;
                    (y_pos, height.max(1.0)) // 至少 1px
                } else {
                    // 负值：从基线向下
                    let height = y_pos - baseline;
                    (baseline, height.max(1.0)) // 至少 1px
                };

                // 绘制柱子
                output.add_command(DrawCmd::Rect {
                    x: bar_x,
                    y: bar_y,
                    width: bar_width,
                    height: bar_height,
                    fill: Some(FillStyle::Color(color.clone())),
                    stroke: None,
                    corner_radius: None,
                });

                // 创建 HitRegion
                let region = HitRegion::from_rect(
                    bar_x,
                    bar_y,
                    bar_width,
                    bar_height,
                    *row_idx,
                    if series_count > 1 { Some(series_idx) } else { None },
                    row_data.clone(),
                );
                hit_regions.push(region);
            }
        }

        Ok((output, hit_regions))
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{Encoding, Mark, Field};
    use deneb_core::{Column, DataType, FieldValue};
    use crate::theme::DefaultTheme;

    fn create_test_data() -> DataTable {
        DataTable::with_columns(vec![
            Column::new(
                "category",
                DataType::Nominal,
                vec![
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("B".to_string()),
                    FieldValue::Text("C".to_string()),
                ],
            ),
            Column::new(
                "value",
                DataType::Quantitative,
                vec![
                    FieldValue::Numeric(10.0),
                    FieldValue::Numeric(20.0),
                    FieldValue::Numeric(15.0),
                ],
            ),
        ])
    }

    fn create_test_spec() -> ChartSpec {
        ChartSpec::builder()
            .mark(Mark::Bar)
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
    fn test_bar_chart_render_basic() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = create_test_data();

        let result = BarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(!output.hit_regions.is_empty());
        assert_eq!(output.hit_regions.len(), 3); // 3 个柱子
    }

    #[test]
    fn test_bar_chart_render_with_title() {
        let spec = ChartSpec::builder()
            .mark(Mark::Bar)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("category"))
                    .y(Field::quantitative("value")),
            )
            .title("Test Chart")
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap();

        let theme = DefaultTheme;
        let data = create_test_data();

        let result = BarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        // 检查标题层是否有内容
        let title_layer = output.layers.get_layer(LayerKind::Title);
        assert!(title_layer.is_some());
        assert!(!title_layer.unwrap().commands.is_empty());
    }

    #[test]
    fn test_bar_chart_render_empty_data() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("category", DataType::Nominal, vec![]),
            Column::new("value", DataType::Quantitative, vec![]),
        ]);

        let result = BarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.hit_regions.is_empty());
        // 背景层应该存在
        assert!(output.layers.get_layer(LayerKind::Background).is_some());
    }

    #[test]
    fn test_bar_chart_render_single_category() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
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

        let result = BarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 1);
    }

    #[test]
    fn test_bar_chart_render_negative_values() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new(
                "category",
                DataType::Nominal,
                vec![
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("B".to_string()),
                ],
            ),
            Column::new(
                "value",
                DataType::Quantitative,
                vec![
                    FieldValue::Numeric(-10.0),
                    FieldValue::Numeric(20.0),
                ],
            ),
        ]);

        let result = BarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 2);
    }

    #[test]
    fn test_bar_chart_render_multi_series() {
        let spec = ChartSpec::builder()
            .mark(Mark::Bar)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("category"))
                    .y(Field::quantitative("value"))
                    .color(Field::nominal("series")),
            )
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap();

        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new(
                "category",
                DataType::Nominal,
                vec![
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("A".to_string()),
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
                    FieldValue::Numeric(25.0),
                ],
            ),
            Column::new(
                "series",
                DataType::Nominal,
                vec![
                    FieldValue::Text("X".to_string()),
                    FieldValue::Text("Y".to_string()),
                    FieldValue::Text("X".to_string()),
                    FieldValue::Text("Y".to_string()),
                ],
            ),
        ]);

        let result = BarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 4);

        // 检查每个柱子都有系列索引
        for region in &output.hit_regions {
            assert!(region.series.is_some());
        }
    }

    #[test]
    fn test_bar_chart_hit_regions() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = create_test_data();

        let result = BarChart::render(&spec, &theme, &data).unwrap();
        let regions = &result.hit_regions;

        assert_eq!(regions.len(), 3);

        // 检查第一个命中区域
        let region = &regions[0];
        assert_eq!(region.index, 0);
        assert!(region.series.is_none()); // 单系列
        assert!(!region.data.is_empty());
    }

    #[test]
    fn test_bar_chart_validate_data_missing_field() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("wrong_field", DataType::Nominal, vec![]),
        ]);

        let result = BarChart::render(&spec, &theme, &data);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_bar_chart_zero_value() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new(
                "category",
                DataType::Nominal,
                vec![FieldValue::Text("A".to_string())],
            ),
            Column::new(
                "value",
                DataType::Quantitative,
                vec![FieldValue::Numeric(0.0)],
            ),
        ]);

        let result = BarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 1);

        // 零值的柱子应该至少有 1px 高度
        let region = &output.hit_regions[0];
        assert!(region.bounds.height >= 1.0);
    }

    #[test]
    fn test_bar_chart_layers() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = create_test_data();

        let result = BarChart::render(&spec, &theme, &data).unwrap();

        // 检查所有必需的层都存在
        assert!(result.layers.get_layer(LayerKind::Background).is_some());
        assert!(result.layers.get_layer(LayerKind::Grid).is_some());
        assert!(result.layers.get_layer(LayerKind::Axis).is_some());
        assert!(result.layers.get_layer(LayerKind::Data).is_some());
    }

    #[test]
    fn test_bar_chart_with_custom_theme() {
        use crate::theme::DarkTheme;

        let spec = create_test_spec();
        let theme = DarkTheme;
        let data = create_test_data();

        let result = BarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(!output.hit_regions.is_empty());
    }
}
