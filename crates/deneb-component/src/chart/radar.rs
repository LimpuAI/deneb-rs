//! 雷达图渲染器
//!
//! 将 ChartSpec 和 DataTable 渲染为雷达图的 Canvas 指令。
//! 使用极坐标系，维度均匀分布在 360°。

use crate::layout::PlotArea;
use crate::spec::ChartSpec;
use crate::theme::Theme;
use crate::chart::ChartOutput;
use crate::error::ComponentError;
use deneb_core::*;

/// RadarChart 渲染器
pub struct RadarChart;

impl RadarChart {
    /// 渲染雷达图
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

        // 2. 计算布局（雷达图使用自己的布局逻辑）
        let plot_area = PlotArea {
            x: theme.margin().left,
            y: theme.margin().top,
            width: spec.width - theme.margin().horizontal(),
            height: spec.height - theme.margin().vertical(),
        };

        // 3. 提取维度和数据
        let dimensions = Self::extract_dimensions(spec, data)?;
        let n_dims = dimensions.len();

        if n_dims < 2 {
            // 少于 2 个维度无法构成多边形
            return Ok(Self::render_empty(spec, theme));
        }

        // 4. 计算极坐标布局参数
        let cx = plot_area.x + plot_area.width / 2.0;
        let cy = plot_area.y + plot_area.height / 2.0;
        let max_radius = (plot_area.width.min(plot_area.height) / 2.0) - 20.0;
        let max_radius = max_radius.max(1.0);

        // 5. 构建 y 值比例尺
        let y_scale = Self::build_y_scale(spec, data);

        // 6. 生成渲染指令
        let mut layers = RenderLayers::new();
        let mut hit_regions = Vec::new();

        // 背景层
        layers.update_layer(LayerKind::Background, super::shared::render_background(spec, theme));

        // 网格层（同心多边形 + 轴线）
        layers.update_layer(LayerKind::Grid, Self::render_grid(
            theme, n_dims, cx, cy, max_radius, &dimensions,
        ));

        // 数据层（多边形）
        let (data_commands, data_regions) = Self::render_data(
            spec, theme, data, &dimensions, &y_scale,
            cx, cy, max_radius,
        )?;
        layers.update_layer(LayerKind::Data, data_commands);
        hit_regions.extend(data_regions);

        // 标题层
        if let Some(title) = &spec.title {
            layers.update_layer(LayerKind::Title, Self::render_title(spec, theme, title, &plot_area));
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

        layers.update_layer(LayerKind::Background, super::shared::render_background(spec, theme));

        if let Some(title) = &spec.title {
            layers.update_layer(LayerKind::Title, Self::render_title(spec, theme, title, &plot_area));
        }

        ChartOutput {
            layers,
            hit_regions: Vec::new(),
        }
    }

    /// 提取维度名称（x 列的唯一值）
    fn extract_dimensions(
        spec: &ChartSpec,
        data: &DataTable,
    ) -> Result<Vec<String>, ComponentError> {
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

        // 保留顺序，去重
        let mut seen = std::collections::HashSet::new();
        let mut dimensions = Vec::new();
        for value in &x_column.values {
            if let Some(text) = value.as_text() {
                if seen.insert(text.to_string()) {
                    dimensions.push(text.to_string());
                }
            }
        }

        Ok(dimensions)
    }

    /// 构建 y 值比例尺（映射到 0-max_radius）
    fn build_y_scale(spec: &ChartSpec, data: &DataTable) -> LinearScale {
        let y_field = spec.encoding.y.as_ref();
        let y_column = y_field.and_then(|f| data.get_column(&f.name));

        let min: f64 = 0.0;
        let mut max: Option<f64> = None;

        if let Some(column) = y_column {
            for value in &column.values {
                if let Some(num) = value.as_numeric() {
                    max = Some(max.map_or(num, |m| m.max(num)));
                }
            }
        }

        let max = max.unwrap_or(100.0);
        let max = max.max(0.001); // 避免除零

        LinearScale::new(min, max, 0.0, 1.0)
    }

    /// 获取维度对应的角度
    fn angle_for_dim(dim_idx: usize, n_dims: usize) -> f64 {
        let start_offset = -std::f64::consts::FRAC_PI_2; // 从 12 点方向开始
        start_offset + (dim_idx as f64) * 2.0 * std::f64::consts::PI / (n_dims as f64)
    }

    /// 渲染背景
    /// 渲染网格（同心多边形 + 轴线 + 维度标签）
    fn render_grid<T: Theme>(
        theme: &T,
        n_dims: usize,
        cx: f64,
        cy: f64,
        max_radius: f64,
        dimensions: &[String],
    ) -> RenderOutput {
        let mut output = RenderOutput::new();

        // 同心多边形（25%, 50%, 75%, 100%）
        let grid_levels = [0.25, 0.5, 0.75, 1.0];

        for &level in &grid_levels {
            let r = max_radius * level;
            let mut segments = Vec::new();

            for dim_idx in 0..n_dims {
                let angle = Self::angle_for_dim(dim_idx, n_dims);
                let px = cx + r * angle.cos();
                let py = cy + r * angle.sin();

                if dim_idx == 0 {
                    segments.push(PathSegment::MoveTo(px, py));
                } else {
                    segments.push(PathSegment::LineTo(px, py));
                }
            }
            segments.push(PathSegment::Close);

            output.add_command(DrawCmd::Path {
                segments,
                fill: None,
                stroke: Some(theme.grid_stroke()),
            });
        }

        // 轴线（中心到每个顶点）
        for dim_idx in 0..n_dims {
            let angle = Self::angle_for_dim(dim_idx, n_dims);
            let px = cx + max_radius * angle.cos();
            let py = cy + max_radius * angle.sin();

            output.add_command(DrawCmd::Path {
                segments: vec![
                    PathSegment::MoveTo(cx, cy),
                    PathSegment::LineTo(px, py),
                ],
                fill: None,
                stroke: Some(theme.grid_stroke()),
            });
        }

        // 维度标签（顶点外侧）
        let text_style = TextStyle::new()
            .with_font_size(theme.tick_font_size())
            .with_font_family(theme.font_family())
            .with_fill(FillStyle::Color(theme.foreground_color().to_string()));

        let label_radius = max_radius + 15.0;

        for (dim_idx, dim_name) in dimensions.iter().enumerate() {
            let angle = Self::angle_for_dim(dim_idx, n_dims);
            let lx = cx + label_radius * angle.cos();
            let ly = cy + label_radius * angle.sin();

            output.add_command(DrawCmd::Text {
                x: lx,
                y: ly,
                content: dim_name.clone(),
                style: text_style.clone(),
                anchor: TextAnchor::Middle,
                baseline: TextBaseline::Middle,
            });
        }

        output
    }

    /// 渲染数据多边形
    fn render_data<T: Theme>(
        spec: &ChartSpec,
        theme: &T,
        data: &DataTable,
        dimensions: &[String],
        y_scale: &LinearScale,
        cx: f64,
        cy: f64,
        max_radius: f64,
    ) -> Result<(RenderOutput, Vec<HitRegion>), ComponentError> {
        let mut output = RenderOutput::new();
        let mut hit_regions = Vec::new();

        let default_name = String::new();
        let x_field_name = spec.encoding.x.as_ref().map(|f| &f.name).unwrap_or(&default_name);
        let y_field_name = spec.encoding.y.as_ref().map(|f| &f.name).unwrap_or(&default_name);
        let color_field = spec.encoding.color.as_ref();
        let color_field_name = color_field.map(|f| f.name.as_str());

        let x_column = data.get_column(x_field_name);
        let y_column = data.get_column(y_field_name);

        // 按系列分组
        let color_column = color_field_name.and_then(|name| data.get_column(name));

        let row_count = data.row_count();
        let mut series_map: std::collections::HashMap<Option<String>, Vec<(String, f64, usize)>> = std::collections::HashMap::new();

        for row_idx in 0..row_count {
            let x_val = x_column
                .and_then(|col| col.get(row_idx))
                .and_then(|v| v.as_text())
                .unwrap_or("")
                .to_string();

            let y_val = y_column
                .and_then(|col| col.get(row_idx))
                .and_then(|v| v.as_numeric())
                .unwrap_or(0.0);

            let series_key = color_column
                .and_then(|col| col.get(row_idx))
                .and_then(|v| v.as_text())
                .map(|s| s.to_string());

            series_map
                .entry(series_key)
                .or_insert_with(Vec::new)
                .push((x_val, y_val, row_idx));
        }

        let series_keys: Vec<_> = series_map.keys().cloned().collect();

        for (series_idx, series_key) in series_keys.iter().enumerate() {
            let entries = series_map.get(series_key).unwrap();
            let n_dims = dimensions.len();

            // 构建 x → y 映射
            let mut value_map: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
            for (x_val, y_val, _) in entries {
                value_map.insert(x_val.clone(), *y_val);
            }

            // 绘制数据多边形
            let mut segments = Vec::new();
            for (dim_idx, dim_name) in dimensions.iter().enumerate() {
                let value = value_map.get(dim_name).copied().unwrap_or(0.0);
                let normalized = y_scale.map(value);
                let r = max_radius * normalized;
                let angle = Self::angle_for_dim(dim_idx, n_dims);
                let px = cx + r * angle.cos();
                let py = cy + r * angle.sin();

                if dim_idx == 0 {
                    segments.push(PathSegment::MoveTo(px, py));
                } else {
                    segments.push(PathSegment::LineTo(px, py));
                }
            }
            segments.push(PathSegment::Close);

            let color = theme.series_color(series_idx).to_string();

            // 半透明填充
            output.add_command(DrawCmd::Path {
                segments: segments.clone(),
                fill: Some(FillStyle::Color(Self::with_alpha(&color, 0.2))),
                stroke: Some(StrokeStyle::Color(color.clone())),
            });

            // 数据点标记 + HitRegion
            for (dim_idx, dim_name) in dimensions.iter().enumerate() {
                let value = value_map.get(dim_name).copied().unwrap_or(0.0);
                let normalized = y_scale.map(value);
                let r = max_radius * normalized;
                let angle = Self::angle_for_dim(dim_idx, n_dims);
                let px = cx + r * angle.cos();
                let py = cy + r * angle.sin();

                // 小圆点
                output.add_command(DrawCmd::Circle {
                    cx: px,
                    cy: py,
                    r: 3.0,
                    fill: Some(FillStyle::Color(color.clone())),
                    stroke: None,
                });

                // HitRegion
                if let Some(&(_, _, row_idx)) = entries.iter().find(|(x, _, _)| *x == *dim_name) {
                    let mut row_data = Vec::new();
                    for column in &data.columns {
                        if let Some(value) = column.get(row_idx) {
                            row_data.push(value.clone());
                        }
                    }

                    hit_regions.push(HitRegion::from_point(
                        px,
                        py,
                        5.0,
                        row_idx,
                        Some(series_idx),
                        row_data,
                    ));
                }
            }
        }

        Ok((output, hit_regions))
    }

    /// 为颜色添加透明度
    fn with_alpha(hex_color: &str, alpha: f64) -> String {
        // 解析 #rrggbb 格式
        if hex_color.starts_with('#') && hex_color.len() == 7 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex_color[1..3], 16),
                u8::from_str_radix(&hex_color[3..5], 16),
                u8::from_str_radix(&hex_color[5..7], 16),
            ) {
                return format!("rgba({},{},{},{:.2})", r, g, b, alpha);
            }
        }
        format!("rgba(128,128,128,{:.2})", alpha)
    }

    /// 渲染标题
    fn render_title<T: Theme>(
        spec: &ChartSpec,
        theme: &T,
        title: &str,
        _plot_area: &PlotArea,
    ) -> RenderOutput {
        let mut output = RenderOutput::new();

        let title_style = TextStyle::new()
            .with_font_size(theme.title_font_size())
            .with_font_family(theme.font_family())
            .with_font_weight(FontWeight::Bold)
            .with_fill(FillStyle::Color(theme.title_color().to_string()));

        output.add_command(DrawCmd::Text {
            x: spec.width / 2.0,
            y: theme.margin().top / 2.0,
            content: title.to_string(),
            style: title_style,
            anchor: TextAnchor::Middle,
            baseline: TextBaseline::Middle,
        });

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{Encoding, Field, Mark};
    use crate::theme::DefaultTheme;
    use deneb_core::{Column, DataType, FieldValue};

    fn create_radar_data() -> DataTable {
        DataTable::with_columns(vec![
            Column::new(
                "dimension",
                DataType::Nominal,
                vec![
                    FieldValue::Text("Speed".to_string()),
                    FieldValue::Text("Power".to_string()),
                    FieldValue::Text("Range".to_string()),
                    FieldValue::Text("Armor".to_string()),
                ],
            ),
            Column::new(
                "value",
                DataType::Quantitative,
                vec![
                    FieldValue::Numeric(80.0),
                    FieldValue::Numeric(60.0),
                    FieldValue::Numeric(90.0),
                    FieldValue::Numeric(40.0),
                ],
            ),
        ])
    }

    fn create_radar_spec() -> ChartSpec {
        ChartSpec::builder()
            .mark(Mark::Radar)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("dimension"))
                    .y(Field::quantitative("value")),
            )
            .width(400.0)
            .height(400.0)
            .build()
            .unwrap()
    }

    #[test]
    fn test_radar_chart_render_basic() {
        let spec = create_radar_spec();
        let theme = DefaultTheme;
        let data = create_radar_data();

        let result = RadarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 4);
        assert!(output.layers.get_layer(LayerKind::Grid).is_some());
        assert!(output.layers.get_layer(LayerKind::Data).is_some());
    }

    #[test]
    fn test_radar_chart_render_empty() {
        let spec = create_radar_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("dimension", DataType::Nominal, vec![]),
            Column::new("value", DataType::Quantitative, vec![]),
        ]);

        let result = RadarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.hit_regions.is_empty());
    }

    #[test]
    fn test_radar_chart_render_two_dimensions() {
        let spec = ChartSpec::builder()
            .mark(Mark::Radar)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("dim"))
                    .y(Field::quantitative("val")),
            )
            .width(400.0)
            .height(400.0)
            .build()
            .unwrap();

        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new(
                "dim",
                DataType::Nominal,
                vec![
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("B".to_string()),
                ],
            ),
            Column::new(
                "val",
                DataType::Quantitative,
                vec![
                    FieldValue::Numeric(50.0),
                    FieldValue::Numeric(80.0),
                ],
            ),
        ]);

        let result = RadarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 2);
    }

    #[test]
    fn test_radar_chart_with_title() {
        let spec = ChartSpec::builder()
            .mark(Mark::Radar)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("dimension"))
                    .y(Field::quantitative("value")),
            )
            .title("Radar Chart")
            .width(400.0)
            .height(400.0)
            .build()
            .unwrap();

        let theme = DefaultTheme;
        let data = create_radar_data();

        let result = RadarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        let title_layer = output.layers.get_layer(LayerKind::Title);
        assert!(title_layer.is_some());
        assert!(!title_layer.unwrap().commands.is_empty());
    }

    #[test]
    fn test_radar_chart_multi_series() {
        let spec = ChartSpec::builder()
            .mark(Mark::Radar)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("dimension"))
                    .y(Field::quantitative("value"))
                    .color(Field::nominal("series")),
            )
            .width(400.0)
            .height(400.0)
            .build()
            .unwrap();

        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new(
                "dimension",
                DataType::Nominal,
                vec![
                    FieldValue::Text("Speed".to_string()),
                    FieldValue::Text("Power".to_string()),
                    FieldValue::Text("Speed".to_string()),
                    FieldValue::Text("Power".to_string()),
                ],
            ),
            Column::new(
                "value",
                DataType::Quantitative,
                vec![
                    FieldValue::Numeric(80.0),
                    FieldValue::Numeric(60.0),
                    FieldValue::Numeric(40.0),
                    FieldValue::Numeric(90.0),
                ],
            ),
            Column::new(
                "series",
                DataType::Nominal,
                vec![
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("B".to_string()),
                    FieldValue::Text("B".to_string()),
                ],
            ),
        ]);

        let result = RadarChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        // 2 series × 2 dimensions = 4 hit regions
        assert_eq!(output.hit_regions.len(), 4);
    }

    #[test]
    fn test_radar_chart_validate_missing_field() {
        let spec = create_radar_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("wrong_field", DataType::Nominal, vec![]),
        ]);

        let result = RadarChart::render(&spec, &theme, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_radar_with_alpha() {
        assert_eq!(
            RadarChart::with_alpha("#1f77b4", 0.2),
            "rgba(31,119,180,0.20)"
        );
        assert_eq!(
            RadarChart::with_alpha("#ff7f0e", 0.5),
            "rgba(255,127,14,0.50)"
        );
    }
}
