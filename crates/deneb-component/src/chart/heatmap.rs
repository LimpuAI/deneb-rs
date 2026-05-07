//! 热力图渲染器
//!
//! 将 ChartSpec 和 DataTable 渲染为热力图的 Canvas 指令。

use crate::layout::{compute_layout, PlotArea};
use crate::spec::ChartSpec;
use crate::theme::Theme;
use crate::chart::ChartOutput;
use crate::error::ComponentError;
use deneb_core::*;

/// HeatmapChart 渲染器
pub struct HeatmapChart;

impl HeatmapChart {
    /// 渲染热力图
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
        let (x_scale, y_scale, color_scale) = Self::build_scales(spec, data, plot_area)?;

        // 4. 生成渲染指令
        let mut layers = RenderLayers::new();
        let mut hit_regions = Vec::new();

        // 背景层
        layers.update_layer(LayerKind::Background, super::shared::render_background_with_border(spec, theme, plot_area));

        // 网格层（热力图无传统网格线，跳过）

        // 数据层（单元格）
        let (data_commands, cell_regions) = Self::render_cells(
            spec,
            theme,
            &x_scale,
            &y_scale,
            &color_scale,
            data,
        )?;
        layers.update_layer(LayerKind::Data, data_commands);
        hit_regions.extend(cell_regions);

        // 轴层
        layers.update_layer(LayerKind::Axis, Self::render_axes(spec, theme, &layout, plot_area, &x_scale, &y_scale));

        // 图例层（色标）
        layers.update_layer(LayerKind::Legend, Self::render_color_bar(spec, theme, &color_scale, plot_area));

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
    ) -> Result<(BandScale, BandScale, LinearScale), ComponentError> {
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

        let color_field = spec.encoding.color.as_ref().ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: "color encoding is required".to_string(),
            }
        })?;

        // X 轴类别
        let x_column = data.get_column(&x_field.name).ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: format!("x field '{}' not found", x_field.name),
            }
        })?;

        let x_categories: Vec<String> = x_column
            .values
            .iter()
            .filter_map(|v| v.as_text().map(|s| s.to_string()))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let x_scale = BandScale::new(
            x_categories,
            plot_area.x,
            plot_area.x + plot_area.width,
            0.05,
        );

        // Y 轴类别
        let y_column = data.get_column(&y_field.name).ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: format!("y field '{}' not found", y_field.name),
            }
        })?;

        let y_categories: Vec<String> = y_column
            .values
            .iter()
            .filter_map(|v| v.as_text().map(|s| s.to_string()))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let y_scale = BandScale::new(
            y_categories,
            plot_area.y,
            plot_area.y + plot_area.height,
            0.05,
        );

        // 颜色比例尺
        let color_column = data.get_column(&color_field.name).ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: format!("color field '{}' not found", color_field.name),
            }
        })?;

        let mut min: Option<f64> = None;
        let mut max: Option<f64> = None;

        for value in &color_column.values {
            if let Some(num) = value.as_numeric() {
                min = Some(min.map_or(num, |m| m.min(num)));
                max = Some(max.map_or(num, |m| m.max(num)));
            }
        }

        let (min, max) = match (min, max) {
            (Some(min), Some(max)) => (min, max),
            _ => (0.0, 100.0),
        };

        let color_scale = LinearScale::new(min, max, 0.0, 1.0);

        Ok((x_scale, y_scale, color_scale))
    }

    /// 将归一化值映射为颜色字符串
    fn color_for_value(t: f64) -> String {
        let t = t.clamp(0.0, 1.0);
        // 色阶: "#313695" (低, 蓝) → "#f7f7f7" (中) → "#a50026" (高, 红)
        let (r, g, b) = if t <= 0.5 {
            let s = t / 0.5;
            let (r0, g0, b0) = (0x31, 0x36, 0x95);
            let (r1, g1, b1) = (0xf7, 0xf7, 0xf7);
            (
                (r0 as f64 + (r1 as f64 - r0 as f64) * s) as u8,
                (g0 as f64 + (g1 as f64 - g0 as f64) * s) as u8,
                (b0 as f64 + (b1 as f64 - b0 as f64) * s) as u8,
            )
        } else {
            let s = (t - 0.5) / 0.5;
            let (r0, g0, b0) = (0xf7, 0xf7, 0xf7);
            let (r1, g1, b1) = (0xa5, 0x00, 0x26);
            (
                (r0 as f64 + (r1 as f64 - r0 as f64) * s) as u8,
                (g0 as f64 + (g1 as f64 - g0 as f64) * s) as u8,
                (b0 as f64 + (b1 as f64 - b0 as f64) * s) as u8,
            )
        };
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    }

    /// 渲染热力图单元格
    fn render_cells<T: Theme>(
        _spec: &ChartSpec,
        _theme: &T,
        x_scale: &BandScale,
        y_scale: &BandScale,
        color_scale: &LinearScale,
        data: &DataTable,
    ) -> Result<(RenderOutput, Vec<HitRegion>), ComponentError> {
        let mut output = RenderOutput::new();
        let mut hit_regions = Vec::new();

        let x_field_name = _spec.encoding.x.as_ref().map(|f| f.name.as_str()).unwrap_or("");
        let y_field_name = _spec.encoding.y.as_ref().map(|f| f.name.as_str()).unwrap_or("");
        let color_field_name = _spec.encoding.color.as_ref().map(|f| f.name.as_str()).unwrap_or("");

        let x_column = data.get_column(x_field_name);
        let y_column = data.get_column(y_field_name);
        let color_column = data.get_column(color_field_name);

        let row_count = data.row_count();
        for row_idx in 0..row_count {
            let x_val = x_column
                .and_then(|col| col.get(row_idx))
                .and_then(|v| v.as_text())
                .unwrap_or("");
            let y_val = y_column
                .and_then(|col| col.get(row_idx))
                .and_then(|v| v.as_text())
                .unwrap_or("");
            let color_val = color_column
                .and_then(|col| col.get(row_idx))
                .and_then(|v| v.as_numeric())
                .unwrap_or(0.0);

            let band_start_x = x_scale.band_start(x_val).unwrap_or(0.0);
            let band_width_x = x_scale.band_width();
            let band_start_y = y_scale.band_start(y_val).unwrap_or(0.0);
            let band_width_y = y_scale.band_width();

            let t = color_scale.map(color_val);
            let color = Self::color_for_value(t);

            output.add_command(DrawCmd::Rect {
                x: band_start_x,
                y: band_start_y,
                width: band_width_x,
                height: band_width_y,
                fill: Some(FillStyle::Color(color)),
                stroke: Some(StrokeStyle::Color("#ffffff".to_string())),
                corner_radius: None,
            });

            // 收集该行所有字段值
            let mut row_data = Vec::new();
            for column in &data.columns {
                if let Some(value) = column.get(row_idx) {
                    row_data.push(value.clone());
                }
            }

            let region = HitRegion::from_rect(
                band_start_x,
                band_start_y,
                band_width_x,
                band_width_y,
                row_idx,
                None,
                row_data,
            );
            hit_regions.push(region);
        }

        Ok((output, hit_regions))
    }

    /// 渲染坐标轴
    fn render_axes<T: Theme>(
        spec: &ChartSpec,
        theme: &T,
        layout: &crate::layout::LayoutResult,
        plot_area: &PlotArea,
        x_scale: &BandScale,
        _y_scale: &BandScale,
    ) -> RenderOutput {
        let mut output = RenderOutput::new();

        // X 轴（底部类别标签）
        if let Some(x_axis) = &layout.x_axis {
            output.add_command(DrawCmd::Path {
                segments: vec![
                    PathSegment::MoveTo(plot_area.x, x_axis.position),
                    PathSegment::LineTo(plot_area.x + plot_area.width, x_axis.position),
                ],
                fill: None,
                stroke: Some(theme.axis_stroke()),
            });

            // X 轴标签：在每个 band 中心
            let tick_size = theme.layout_config().tick_length;
            let text_style = TextStyle::new()
                .with_font_size(theme.tick_font_size())
                .with_font_family(theme.font_family())
                .with_fill(FillStyle::Color(theme.foreground_color().to_string()));

            // 用 x_scale 的类别来画标签
            for cat_idx in 0..x_scale.step_width().ceil() as usize {
                // 获取 band_center
                // 直接用 x_axis 的 tick_positions 和 tick_labels
                if cat_idx < x_axis.tick_positions.len() && cat_idx < x_axis.tick_labels.len() {
                    let tick_pos = x_axis.tick_positions[cat_idx];
                    let label = &x_axis.tick_labels[cat_idx];

                    output.add_command(DrawCmd::Path {
                        segments: vec![
                            PathSegment::MoveTo(tick_pos, x_axis.position),
                            PathSegment::LineTo(tick_pos, x_axis.position + tick_size),
                        ],
                        fill: None,
                        stroke: Some(theme.axis_stroke()),
                    });

                    output.add_command(DrawCmd::Text {
                        x: tick_pos,
                        y: x_axis.position + tick_size + 5.0,
                        content: label.clone(),
                        style: text_style.clone(),
                        anchor: TextAnchor::Middle,
                        baseline: TextBaseline::Top,
                    });
                }
            }

            if let Some(x_field) = &spec.encoding.x {
                let title = x_field.title.as_ref().unwrap_or(&x_field.name);
                let label_style = TextStyle::new()
                    .with_font_size(theme.label_font_size())
                    .with_font_family(theme.font_family())
                    .with_fill(FillStyle::Color(theme.foreground_color().to_string()));

                output.add_command(DrawCmd::Text {
                    x: plot_area.x + plot_area.width / 2.0,
                    y: plot_area.y + plot_area.height + theme.margin().bottom - 5.0,
                    content: title.clone(),
                    style: label_style,
                    anchor: TextAnchor::Middle,
                    baseline: TextBaseline::Bottom,
                });
            }
        }

        // Y 轴（左侧类别标签）
        if let Some(y_axis) = &layout.y_axis {
            output.add_command(DrawCmd::Path {
                segments: vec![
                    PathSegment::MoveTo(y_axis.position, plot_area.y),
                    PathSegment::LineTo(y_axis.position, plot_area.y + plot_area.height),
                ],
                fill: None,
                stroke: Some(theme.axis_stroke()),
            });

            let tick_size = theme.layout_config().tick_length;
            let text_style = TextStyle::new()
                .with_font_size(theme.tick_font_size())
                .with_font_family(theme.font_family())
                .with_fill(FillStyle::Color(theme.foreground_color().to_string()));

            for (tick_pos, label) in y_axis.tick_positions.iter().zip(y_axis.tick_labels.iter()) {
                output.add_command(DrawCmd::Path {
                    segments: vec![
                        PathSegment::MoveTo(y_axis.position - tick_size, *tick_pos),
                        PathSegment::LineTo(y_axis.position, *tick_pos),
                    ],
                    fill: None,
                    stroke: Some(theme.axis_stroke()),
                });

                output.add_command(DrawCmd::Text {
                    x: y_axis.position - tick_size - 5.0,
                    y: *tick_pos,
                    content: label.clone(),
                    style: text_style.clone(),
                    anchor: TextAnchor::End,
                    baseline: TextBaseline::Middle,
                });
            }

            if let Some(y_field) = &spec.encoding.y {
                let title = y_field.title.as_ref().unwrap_or(&y_field.name);
                let label_style = TextStyle::new()
                    .with_font_size(theme.label_font_size())
                    .with_font_family(theme.font_family())
                    .with_fill(FillStyle::Color(theme.foreground_color().to_string()));

                output.add_command(DrawCmd::Text {
                    x: plot_area.x - theme.margin().left + 5.0,
                    y: plot_area.y + plot_area.height / 2.0,
                    content: title.clone(),
                    style: label_style,
                    anchor: TextAnchor::Middle,
                    baseline: TextBaseline::Top,
                });
            }
        }

        output
    }

    /// 渲染色标（图例）
    fn render_color_bar<T: Theme>(
        _spec: &ChartSpec,
        theme: &T,
        color_scale: &LinearScale,
        plot_area: &PlotArea,
    ) -> RenderOutput {
        let mut output = RenderOutput::new();

        let bar_width = 15.0;
        let bar_height = plot_area.height;
        let bar_x = plot_area.x + plot_area.width + 10.0;
        let bar_y = plot_area.y;

        // 使用渐变绘制色标
        let gradient = Gradient {
            kind: GradientKind::Linear {
                x0: bar_x,
                y0: bar_y + bar_height,
                x1: bar_x,
                y1: bar_y,
            },
            stops: vec![
                GradientStop::new(0.0, "#313695".to_string()),
                GradientStop::new(0.5, "#f7f7f7".to_string()),
                GradientStop::new(1.0, "#a50026".to_string()),
            ],
        };

        output.add_command(DrawCmd::Rect {
            x: bar_x,
            y: bar_y,
            width: bar_width,
            height: bar_height,
            fill: Some(FillStyle::Gradient(gradient)),
            stroke: Some(StrokeStyle::Color(theme.foreground_color().to_string())),
            corner_radius: None,
        });

        // 色标刻度标签
        let (domain_min, domain_max) = color_scale.domain();
        let text_style = TextStyle::new()
            .with_font_size(theme.tick_font_size())
            .with_font_family(theme.font_family())
            .with_fill(FillStyle::Color(theme.foreground_color().to_string()));

        let n_ticks = 3;
        for i in 0..=n_ticks {
            let t = i as f64 / n_ticks as f64;
            let y_pos = bar_y + bar_height * (1.0 - t);
            let value = domain_min + t * (domain_max - domain_min);

            // 刻度线
            output.add_command(DrawCmd::Path {
                segments: vec![
                    PathSegment::MoveTo(bar_x + bar_width, y_pos),
                    PathSegment::LineTo(bar_x + bar_width + 4.0, y_pos),
                ],
                fill: None,
                stroke: Some(StrokeStyle::Color(theme.foreground_color().to_string())),
            });

            // 标签
            let label = if value.fract() == 0.0 && value.abs() < 1e10 {
                format!("{}", value as i64)
            } else {
                format!("{:.1}", value)
            };

            output.add_command(DrawCmd::Text {
                x: bar_x + bar_width + 6.0,
                y: y_pos,
                content: label,
                style: text_style.clone(),
                anchor: TextAnchor::Start,
                baseline: TextBaseline::Middle,
            });
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{Encoding, Field, Mark};
    use crate::theme::DefaultTheme;
    use deneb_core::{Column, DataType, FieldValue};

    fn create_heatmap_data() -> DataTable {
        DataTable::with_columns(vec![
            Column::new(
                "x_cat",
                DataType::Nominal,
                vec![
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("B".to_string()),
                    FieldValue::Text("B".to_string()),
                ],
            ),
            Column::new(
                "y_cat",
                DataType::Nominal,
                vec![
                    FieldValue::Text("X".to_string()),
                    FieldValue::Text("Y".to_string()),
                    FieldValue::Text("X".to_string()),
                    FieldValue::Text("Y".to_string()),
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
                ],
            ),
        ])
    }

    fn create_heatmap_spec() -> ChartSpec {
        ChartSpec::builder()
            .mark(Mark::Heatmap)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("x_cat"))
                    .y(Field::nominal("y_cat"))
                    .color(Field::quantitative("value")),
            )
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap()
    }

    #[test]
    fn test_heatmap_render_basic() {
        let spec = create_heatmap_spec();
        let theme = DefaultTheme;
        let data = create_heatmap_data();

        let result = HeatmapChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 4);
        assert!(output.layers.get_layer(LayerKind::Data).is_some());
        assert!(output.layers.get_layer(LayerKind::Legend).is_some());
    }

    #[test]
    fn test_heatmap_render_empty_data() {
        let spec = create_heatmap_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("x_cat", DataType::Nominal, vec![]),
            Column::new("y_cat", DataType::Nominal, vec![]),
            Column::new("value", DataType::Quantitative, vec![]),
        ]);

        let result = HeatmapChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.hit_regions.is_empty());
        assert!(output.layers.get_layer(LayerKind::Background).is_some());
    }

    #[test]
    fn test_heatmap_render_with_title() {
        let spec = ChartSpec::builder()
            .mark(Mark::Heatmap)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("x_cat"))
                    .y(Field::nominal("y_cat"))
                    .color(Field::quantitative("value")),
            )
            .title("Test Heatmap")
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap();

        let theme = DefaultTheme;
        let data = create_heatmap_data();

        let result = HeatmapChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        let title_layer = output.layers.get_layer(LayerKind::Title);
        assert!(title_layer.is_some());
        assert!(!title_layer.unwrap().commands.is_empty());
    }

    #[test]
    fn test_heatmap_color_for_value() {
        // 低值 → 蓝色
        let low = HeatmapChart::color_for_value(0.0);
        assert!(low.starts_with('#'));
        // 应接近 "#313695"
        assert_eq!(low, "#313695");

        // 高值 → 红色
        let high = HeatmapChart::color_for_value(1.0);
        assert_eq!(high, "#a50026");

        // 中值
        let mid = HeatmapChart::color_for_value(0.5);
        assert_eq!(mid, "#f7f7f7");
    }

    #[test]
    fn test_heatmap_validate_missing_field() {
        let spec = create_heatmap_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("wrong_field", DataType::Nominal, vec![]),
        ]);

        let result = HeatmapChart::render(&spec, &theme, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_heatmap_single_cell() {
        let spec = create_heatmap_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new(
                "x_cat",
                DataType::Nominal,
                vec![FieldValue::Text("A".to_string())],
            ),
            Column::new(
                "y_cat",
                DataType::Nominal,
                vec![FieldValue::Text("X".to_string())],
            ),
            Column::new(
                "value",
                DataType::Quantitative,
                vec![FieldValue::Numeric(42.0)],
            ),
        ]);

        let result = HeatmapChart::render(&spec, &theme, &data);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 1);
    }

    #[test]
    fn test_heatmap_all_same_value() {
        let spec = create_heatmap_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new(
                "x_cat",
                DataType::Nominal,
                vec![
                    FieldValue::Text("A".to_string()),
                    FieldValue::Text("B".to_string()),
                ],
            ),
            Column::new(
                "y_cat",
                DataType::Nominal,
                vec![
                    FieldValue::Text("X".to_string()),
                    FieldValue::Text("X".to_string()),
                ],
            ),
            Column::new(
                "value",
                DataType::Quantitative,
                vec![
                    FieldValue::Numeric(50.0),
                    FieldValue::Numeric(50.0),
                ],
            ),
        ]);

        let result = HeatmapChart::render(&spec, &theme, &data);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 2);
    }
}
