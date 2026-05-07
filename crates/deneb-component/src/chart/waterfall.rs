//! 瀑布图渲染器
//!
//! 将 ChartSpec 和 DataTable 渲染为瀑布图的 Canvas 指令。
//! 累计基线计算，正值绿色、负值红色，最后一根柱为合计。

use crate::layout::{compute_layout, LayoutResult, PlotArea};
use crate::spec::ChartSpec;
use crate::theme::Theme;
use crate::chart::ChartOutput;
use crate::error::ComponentError;
use deneb_core::*;

/// WaterfallChart 渲染器
pub struct WaterfallChart;

/// 瀑布图单条数据
struct WaterfallBar {
    /// 类别标签
    label: String,
    /// 增量值
    value: f64,
    /// 柱子底边（像素空间之前，数据空间的 running baseline）
    baseline: f64,
    /// 是否为合计柱
    is_total: bool,
}

impl WaterfallChart {
    /// 渲染瀑布图
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

        // 2. 提取数据
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
                reason: format!("x field '{}' not found in data", x_field.name),
            }
        })?;

        let y_column = data.get_column(&y_field.name).ok_or_else(|| {
            ComponentError::InvalidConfig {
                reason: format!("y field '{}' not found in data", y_field.name),
            }
        })?;

        // 3. 计算瀑布数据（含合计柱）
        let bars = Self::compute_waterfall(x_column, y_column, data.row_count());

        if bars.is_empty() {
            return Ok(Self::render_empty(spec, theme));
        }

        // 4. 计算布局
        let layout = compute_layout(spec, theme, data);
        let plot_area = &layout.plot_area;

        // 5. 构建 Scale
        let (x_scale, y_scale) = Self::build_scales(&bars, plot_area)?;

        // 6. 生成渲染指令
        let mut layers = RenderLayers::new();
        let mut hit_regions = Vec::new();

        // 背景层
        layers.update_layer(LayerKind::Background, super::shared::render_background_with_border(spec, theme, plot_area));

        // 网格层
        if let Some(y_axis) = &layout.y_axis {
            layers.update_layer(LayerKind::Grid, super::shared::render_grid_horizontal(theme, &y_axis.tick_positions, plot_area));
        }

        // 数据层
        let (data_commands, bar_regions) = Self::render_bars(
            theme, &x_scale, &y_scale, &bars, plot_area, data,
        )?;
        layers.update_layer(LayerKind::Data, data_commands);
        hit_regions.extend(bar_regions);

        // 轴层
        layers.update_layer(LayerKind::Axis, Self::render_axes(spec, theme, &layout, plot_area, &bars, &x_scale));

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

    /// 计算瀑布数据：运行基线 + 合计柱
    fn compute_waterfall(
        x_column: &Column,
        y_column: &Column,
        row_count: usize,
    ) -> Vec<WaterfallBar> {
        let mut bars = Vec::new();
        let mut running = 0.0;

        for i in 0..row_count {
            let label = x_column
                .get(i)
                .and_then(|v| v.as_text())
                .unwrap_or("")
                .to_string();

            let value = y_column
                .get(i)
                .and_then(|v| v.as_numeric())
                .unwrap_or(0.0);

            bars.push(WaterfallBar {
                label,
                value,
                baseline: running,
                is_total: false,
            });

            running += value;
        }

        // 合计柱
        bars.push(WaterfallBar {
            label: "Total".to_string(),
            value: running,
            baseline: 0.0,
            is_total: true,
        });

        bars
    }

    /// 构建比例尺
    fn build_scales(
        bars: &[WaterfallBar],
        plot_area: &PlotArea,
    ) -> Result<(BandScale, LinearScale), ComponentError> {
        // X 轴：BandScale（所有类别 + 合计）
        let categories: Vec<String> = bars.iter().map(|b| b.label.clone()).collect();
        let x_scale = BandScale::new(
            categories,
            plot_area.x,
            plot_area.x + plot_area.width,
            0.1,
        );

        // Y 轴：LinearScale，从 0 开始
        let mut min = 0.0f64;
        let mut max = 0.0f64;

        for bar in bars {
            let top;
            let bottom;
            if bar.is_total {
                bottom = 0.0;
                top = bar.value;
            } else {
                bottom = bar.baseline;
                top = bar.baseline + bar.value;
            }
            min = min.min(bottom).min(top);
            max = max.max(bottom).max(top);
        }

        // Y 轴必须包含 0
        min = min.min(0.0);
        max = max.max(0.0);

        let y_scale = LinearScale::new(
            min,
            max,
            plot_area.y + plot_area.height,
            plot_area.y,
        );

        Ok((x_scale, y_scale))
    }

    /// 渲染瀑布柱子
    fn render_bars<T: Theme>(
        theme: &T,
        x_scale: &BandScale,
        y_scale: &LinearScale,
        bars: &[WaterfallBar],
        _plot_area: &PlotArea,
        data: &DataTable,
    ) -> Result<(RenderOutput, Vec<HitRegion>), ComponentError> {
        let mut output = RenderOutput::new();
        let mut hit_regions = Vec::new();

        let baseline_zero = y_scale.map(0.0);

        for (i, bar) in bars.iter().enumerate() {
            let band_start = x_scale.band_start(&bar.label).ok_or_else(|| {
                ComponentError::InvalidConfig {
                    reason: format!("category '{}' not found in x scale", bar.label),
                }
            })?;

            let band_width = x_scale.band_width();

            let (bar_y, bar_height, color) = if bar.is_total {
                // 合计柱从 0 到 total
                let top = y_scale.map(bar.value);
                let height = (baseline_zero - top).abs().max(1.0);
                let y = if bar.value >= 0.0 { top } else { baseline_zero };
                (y, height, theme.foreground_color().to_string())
            } else {
                // 增量柱
                let baseline_px = y_scale.map(bar.baseline);
                let top_px = y_scale.map(bar.baseline + bar.value);

                if bar.value >= 0.0 {
                    // 正值：绿色
                    let height = (baseline_px - top_px).max(1.0);
                    (top_px, height, "#4caf50".to_string())
                } else {
                    // 负值：红色
                    let height = (top_px - baseline_px).max(1.0);
                    (baseline_px, height, "#f44336".to_string())
                }
            };

            output.add_command(DrawCmd::Rect {
                x: band_start,
                y: bar_y,
                width: band_width,
                height: bar_height,
                fill: Some(FillStyle::Color(color)),
                stroke: None,
                corner_radius: None,
            });

            // 收集行数据
            let row_data = if i < data.row_count() {
                data.columns
                    .iter()
                    .filter_map(|col| col.values.get(i).cloned())
                    .collect()
            } else {
                vec![FieldValue::Numeric(bar.value)]
            };

            let region = HitRegion::from_rect(
                band_start,
                bar_y,
                band_width,
                bar_height,
                i,
                None,
                row_data,
            );
            hit_regions.push(region);
        }

        Ok((output, hit_regions))
    }

    /// 渲染坐标轴（使用瀑布数据的标签覆盖 layout 的 X 轴标签）
    fn render_axes<T: Theme>(
        spec: &ChartSpec,
        theme: &T,
        layout: &LayoutResult,
        plot_area: &PlotArea,
        bars: &[WaterfallBar],
        x_scale: &BandScale,
    ) -> RenderOutput {
        let mut output = RenderOutput::new();

        // X 轴
        if let Some(x_axis) = &layout.x_axis {
            output.add_command(DrawCmd::Path {
                segments: vec![
                    PathSegment::MoveTo(plot_area.x, x_axis.position),
                    PathSegment::LineTo(plot_area.x + plot_area.width, x_axis.position),
                ],
                fill: None,
                stroke: Some(theme.axis_stroke()),
            });

            let tick_size = theme.layout_config().tick_length;
            // 使用瀑布柱的标签
            for bar in bars {
                let center_x = x_scale.band_center(&bar.label).unwrap_or_else(|| {
                    plot_area.x
                        + (bars.iter().position(|b| b.label == bar.label).unwrap_or(0) as f64 + 0.5)
                            * plot_area.width
                            / bars.len() as f64
                });

                output.add_command(DrawCmd::Path {
                    segments: vec![
                        PathSegment::MoveTo(center_x, x_axis.position),
                        PathSegment::LineTo(center_x, x_axis.position + tick_size),
                    ],
                    fill: None,
                    stroke: Some(theme.axis_stroke()),
                });

                let text_style = TextStyle::new()
                    .with_font_size(theme.tick_font_size())
                    .with_font_family(theme.font_family())
                    .with_fill(FillStyle::Color(theme.foreground_color().to_string()));

                output.add_command(DrawCmd::Text {
                    x: center_x,
                    y: x_axis.position + tick_size + 5.0,
                    content: bar.label.clone(),
                    style: text_style,
                    anchor: TextAnchor::Middle,
                    baseline: TextBaseline::Top,
                });
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

        // Y 轴
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
            for (tick_pos, label) in y_axis.tick_positions.iter().zip(y_axis.tick_labels.iter()) {
                output.add_command(DrawCmd::Path {
                    segments: vec![
                        PathSegment::MoveTo(y_axis.position - tick_size, *tick_pos),
                        PathSegment::LineTo(y_axis.position, *tick_pos),
                    ],
                    fill: None,
                    stroke: Some(theme.axis_stroke()),
                });

                let text_style = TextStyle::new()
                    .with_font_size(theme.tick_font_size())
                    .with_font_family(theme.font_family())
                    .with_fill(FillStyle::Color(theme.foreground_color().to_string()));

                output.add_command(DrawCmd::Text {
                    x: y_axis.position - tick_size - 5.0,
                    y: *tick_pos,
                    content: label.clone(),
                    style: text_style,
                    anchor: TextAnchor::End,
                    baseline: TextBaseline::Middle,
                });
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{Encoding, Mark, Field};
    use crate::theme::DefaultTheme;
    use deneb_core::{Column, DataType, FieldValue};

    fn create_test_data() -> DataTable {
        DataTable::with_columns(vec![
            Column::new(
                "category",
                DataType::Nominal,
                vec![
                    FieldValue::Text("Q1".to_string()),
                    FieldValue::Text("Q2".to_string()),
                    FieldValue::Text("Q3".to_string()),
                    FieldValue::Text("Q4".to_string()),
                ],
            ),
            Column::new(
                "value",
                DataType::Quantitative,
                vec![
                    FieldValue::Numeric(10.0),
                    FieldValue::Numeric(-5.0),
                    FieldValue::Numeric(15.0),
                    FieldValue::Numeric(8.0),
                ],
            ),
        ])
    }

    fn create_test_spec() -> ChartSpec {
        ChartSpec::builder()
            .mark(Mark::Waterfall)
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
    fn test_waterfall_render_basic() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = create_test_data();

        let result = WaterfallChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        // 4 bars + 1 total = 5 hit regions
        assert_eq!(output.hit_regions.len(), 5);
    }

    #[test]
    fn test_waterfall_render_empty_data() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("category", DataType::Nominal, vec![]),
            Column::new("value", DataType::Quantitative, vec![]),
        ]);

        let result = WaterfallChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.hit_regions.is_empty());
        assert!(output.layers.get_layer(LayerKind::Background).is_some());
    }

    #[test]
    fn test_waterfall_all_positive() {
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
                    FieldValue::Numeric(10.0),
                    FieldValue::Numeric(20.0),
                ],
            ),
        ]);

        let result = WaterfallChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        // 2 bars + 1 total
        assert_eq!(output.hit_regions.len(), 3);
    }

    #[test]
    fn test_waterfall_all_negative() {
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
                    FieldValue::Numeric(-5.0),
                ],
            ),
        ]);

        let result = WaterfallChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.hit_regions.len(), 3);
    }

    #[test]
    fn test_waterfall_validate_data_missing_field() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("wrong_field", DataType::Nominal, vec![]),
        ]);

        let result = WaterfallChart::render(&spec, &theme, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_waterfall_layers() {
        let spec = create_test_spec();
        let theme = DefaultTheme;
        let data = create_test_data();

        let result = WaterfallChart::render(&spec, &theme, &data).unwrap();

        assert!(result.layers.get_layer(LayerKind::Background).is_some());
        assert!(result.layers.get_layer(LayerKind::Grid).is_some());
        assert!(result.layers.get_layer(LayerKind::Axis).is_some());
        assert!(result.layers.get_layer(LayerKind::Data).is_some());
    }
}
