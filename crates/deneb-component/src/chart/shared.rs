//! Shared rendering helpers for chart types
//!
//! Extracts duplicated background, grid, axes, and title rendering logic
//! that is identical across multiple chart types.

use crate::layout::{LayoutResult, PlotArea};
use crate::spec::ChartSpec;
use crate::theme::Theme;
use deneb_core::*;

/// Render chart background only (full-width rect, no plot area border).
///
/// Used by charts that don't need a plot area border frame
/// (line, scatter, area, pie, radar).
pub fn render_background<T: Theme>(spec: &ChartSpec, theme: &T) -> RenderOutput {
    RenderOutput::from_commands(vec![DrawCmd::Rect {
        x: 0.0,
        y: 0.0,
        width: spec.width,
        height: spec.height,
        fill: Some(FillStyle::Color(theme.background_color().to_string())),
        stroke: None,
        corner_radius: None,
        corner_radii: None,
        id: None,
}])
}

/// Render chart background with plot area border.
///
/// Used by cartesian charts that draw a frame around the plot area
/// (bar, histogram, box_plot, strip, candlestick, waterfall, heatmap, etc.).
pub fn render_background_with_border<T: Theme>(
    spec: &ChartSpec,
    theme: &T,
    plot_area: &PlotArea,
) -> RenderOutput {
    let mut output = RenderOutput::new();

    // Full chart background
    output.add_command(DrawCmd::Rect {
        x: 0.0,
        y: 0.0,
        width: spec.width,
        height: spec.height,
        fill: Some(FillStyle::Color(theme.background_color().to_string())),
        stroke: None,
        corner_radius: None,
        corner_radii: None,
        id: None,
});

    // Plot area border
    output.add_command(DrawCmd::Rect {
        x: plot_area.x,
        y: plot_area.y,
        width: plot_area.width,
        height: plot_area.height,
        fill: None,
        stroke: Some(StrokeStyle::Color(theme.foreground_color().to_string())),
        corner_radius: None,
        corner_radii: None,
        id: None,
});

    output
}

/// Render horizontal grid lines (Y-axis tick positions).
///
/// Used by bar-style charts that only draw horizontal grid lines.
pub fn render_grid_horizontal<T: Theme>(
    theme: &T,
    tick_positions: &[f64],
    plot_area: &PlotArea,
) -> RenderOutput {
    let mut output = RenderOutput::new();

    for &y_pos in tick_positions {
        output.add_command(DrawCmd::Path {
            segments: vec![
                PathSegment::MoveTo(plot_area.x, y_pos),
                PathSegment::LineTo(plot_area.x + plot_area.width, y_pos),
            ],
            fill: None,
            stroke: Some(theme.grid_stroke()),
            dash: None,
            line_cap: None,
            id: None,
});
    }

    output
}

/// Render standard cartesian axes with tick marks, labels, and optional axis titles.
///
/// Returns `(grid_output, axis_output)` tuple.
/// Grid includes both vertical (X ticks) and horizontal (Y ticks) grid lines.
/// Axes include axis lines, tick labels, but no tick marks and no axis titles.
///
/// Used by line, scatter, area charts.
pub fn render_cartesian_grid_and_axes<T: Theme>(
    layout: &LayoutResult,
    theme: &T,
) -> (RenderOutput, RenderOutput) {
    let mut grid_commands = Vec::new();
    let mut axis_commands = Vec::new();

    // X axis: vertical grid lines + axis line + tick labels
    if let Some(x_axis) = &layout.x_axis {
        // Vertical grid lines
        for &pos in &x_axis.tick_positions {
            grid_commands.push(DrawCmd::Path {
                segments: vec![
                    PathSegment::MoveTo(pos, layout.plot_area.y),
                    PathSegment::LineTo(pos, layout.plot_area.y + layout.plot_area.height),
                ],
                fill: None,
                stroke: Some(theme.grid_stroke().clone()),
                dash: None,
                line_cap: None,
                id: None,
});
        }

        // Axis line
        axis_commands.push(DrawCmd::Path {
            segments: vec![
                PathSegment::MoveTo(layout.plot_area.x, x_axis.position),
                PathSegment::LineTo(
                    layout.plot_area.x + layout.plot_area.width,
                    x_axis.position,
                ),
            ],
            fill: None,
            stroke: Some(theme.axis_stroke().clone()),
            dash: None,
            line_cap: None,
            id: None,
});

        // Tick labels (no tick marks)
        let tick_size = theme.layout_config().tick_length;
        for (i, &pos) in x_axis.tick_positions.iter().enumerate() {
            if let Some(label) = x_axis.tick_labels.get(i) {
                axis_commands.push(DrawCmd::Text {
                    x: pos,
                    y: x_axis.position + tick_size + 2.0,
                    content: label.clone(),
                    style: TextStyle::new()
                        .with_font_size(theme.tick_font_size())
                        .with_font_family(theme.font_family())
                        .with_fill(FillStyle::Color(theme.foreground_color().to_string())),
                    anchor: TextAnchor::Middle,
                    baseline: TextBaseline::Top,
                });
            }
        }
    }

    // Y axis: horizontal grid lines + axis line + tick labels
    if let Some(y_axis) = &layout.y_axis {
        // Horizontal grid lines
        for &pos in &y_axis.tick_positions {
            grid_commands.push(DrawCmd::Path {
                segments: vec![
                    PathSegment::MoveTo(layout.plot_area.x, pos),
                    PathSegment::LineTo(layout.plot_area.x + layout.plot_area.width, pos),
                ],
                fill: None,
                stroke: Some(theme.grid_stroke().clone()),
                dash: None,
                line_cap: None,
                id: None,
});
        }

        // Axis line
        axis_commands.push(DrawCmd::Path {
            segments: vec![
                PathSegment::MoveTo(y_axis.position, layout.plot_area.y),
                PathSegment::LineTo(
                    y_axis.position,
                    layout.plot_area.y + layout.plot_area.height,
                ),
            ],
            fill: None,
            stroke: Some(theme.axis_stroke().clone()),
            dash: None,
            line_cap: None,
            id: None,
});

        // Tick labels (no tick marks)
        let tick_size = theme.layout_config().tick_length;
        for (i, &pos) in y_axis.tick_positions.iter().enumerate() {
            if let Some(label) = y_axis.tick_labels.get(i) {
                axis_commands.push(DrawCmd::Text {
                    x: y_axis.position - tick_size - 2.0,
                    y: pos,
                    content: label.clone(),
                    style: TextStyle::new()
                        .with_font_size(theme.tick_font_size())
                        .with_font_family(theme.font_family())
                        .with_fill(FillStyle::Color(theme.foreground_color().to_string())),
                    anchor: TextAnchor::End,
                    baseline: TextBaseline::Middle,
                });
            }
        }
    }

    (
        RenderOutput::from_commands(grid_commands),
        RenderOutput::from_commands(axis_commands),
    )
}

/// Render standard axes with tick marks, tick labels, and optional axis titles.
///
/// When `include_y_axis_title` is true, draws a Y-axis field title
/// (used by bar, box_plot, strip). When false, omits it
/// (used by histogram, candlestick).
pub fn render_axes<T: Theme>(
    spec: &ChartSpec,
    theme: &T,
    layout: &LayoutResult,
    plot_area: &PlotArea,
    include_y_axis_title: bool,
) -> RenderOutput {
    let mut output = RenderOutput::new();

    // X axis
    if let Some(x_axis) = &layout.x_axis {
        // Axis line
        output.add_command(DrawCmd::Path {
            segments: vec![
                PathSegment::MoveTo(plot_area.x, x_axis.position),
                PathSegment::LineTo(plot_area.x + plot_area.width, x_axis.position),
            ],
            fill: None,
            stroke: Some(theme.axis_stroke()),
            dash: None,
            line_cap: None,
            id: None,
});

        // Tick marks and labels
        let tick_size = theme.layout_config().tick_length;
        for (tick_pos, label) in x_axis.tick_positions.iter().zip(x_axis.tick_labels.iter()) {
            // Tick mark
            output.add_command(DrawCmd::Path {
                segments: vec![
                    PathSegment::MoveTo(*tick_pos, x_axis.position),
                    PathSegment::LineTo(*tick_pos, x_axis.position + tick_size),
                ],
                fill: None,
                stroke: Some(theme.axis_stroke()),
                dash: None,
                line_cap: None,
                id: None,
});

            // Tick label
            let text_style = TextStyle::new()
                .with_font_size(theme.tick_font_size())
                .with_font_family(theme.font_family())
                .with_fill(FillStyle::Color(theme.foreground_color().to_string()));

            output.add_command(DrawCmd::Text {
                x: *tick_pos,
                y: x_axis.position + tick_size + 5.0,
                content: label.clone(),
                style: text_style,
                anchor: TextAnchor::Middle,
                baseline: TextBaseline::Top,
            });
        }

        // X axis title
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

    // Y axis
    if let Some(y_axis) = &layout.y_axis {
        // Axis line
        output.add_command(DrawCmd::Path {
            segments: vec![
                PathSegment::MoveTo(y_axis.position, plot_area.y),
                PathSegment::LineTo(y_axis.position, plot_area.y + plot_area.height),
            ],
            fill: None,
            stroke: Some(theme.axis_stroke()),
            dash: None,
            line_cap: None,
            id: None,
});

        // Tick marks and labels
        let tick_size = theme.layout_config().tick_length;
        for (tick_pos, label) in y_axis.tick_positions.iter().zip(y_axis.tick_labels.iter()) {
            // Tick mark
            output.add_command(DrawCmd::Path {
                segments: vec![
                    PathSegment::MoveTo(y_axis.position - tick_size, *tick_pos),
                    PathSegment::LineTo(y_axis.position, *tick_pos),
                ],
                fill: None,
                stroke: Some(theme.axis_stroke()),
                dash: None,
                line_cap: None,
                id: None,
});

            // Tick label
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

        // Y axis title (optional) — 竖排(270°,自下而上阅读,行业惯例)
        //
        // 水平排布时标题无法塞进左侧 margin(与刻度标签重叠或越界被宿主
        // 裁剪),竖排条带宽度 ≈ 字高,margin.left 扣除刻度标签后足以容纳。
        // anchor/baseline 取 Middle/Middle:旋转围绕 anchor 点,文字中心
        // 恰好落在 (x, plot 中线)。
        if include_y_axis_title {
            if let Some(y_field) = &spec.encoding.y {
                let title = y_field.title.as_ref().unwrap_or(&y_field.name);
                let label_style = TextStyle::new()
                    .with_font_size(theme.label_font_size())
                    .with_font_family(theme.font_family())
                    .with_fill(FillStyle::Color(theme.foreground_color().to_string()))
                    .with_rotation(270.0);

                output.add_command(DrawCmd::Text {
                    x: plot_area.x - theme.margin().left + 8.0,
                    y: plot_area.y + plot_area.height / 2.0,
                    content: title.clone(),
                    style: label_style,
                    anchor: TextAnchor::Middle,
                    baseline: TextBaseline::Middle,
                });
            }
        }
    }

    output
}

/// Render chart title centered above the plot area.
///
/// Used by almost all chart types (bar, histogram, box_plot, strip,
/// candlestick, waterfall, heatmap, sankey, chord, contour, line, scatter, area).
pub fn render_title<T: Theme>(theme: &T, title: &str, plot_area: &PlotArea) -> RenderOutput {
    let mut output = RenderOutput::new();

    let title_style = TextStyle::new()
        .with_font_size(theme.title_font_size())
        .with_font_family(theme.font_family())
        .with_font_weight(FontWeight::Bold)
        .with_fill(FillStyle::Color(theme.title_color().to_string()));

    output.add_command(DrawCmd::Text {
        x: plot_area.x + plot_area.width / 2.0,
        y: plot_area.y - 10.0,
        content: title.to_string(),
        style: title_style,
        anchor: TextAnchor::Middle,
        baseline: TextBaseline::Bottom,
    });

    output
}

/// 多系列图例层(T26)— 色块 + 标签横排,绘制区上方右侧(label 字号档)。
///
/// 仅在系列数 ≥ 2 时由各 mark 调用(单系列图例是噪音);条目宽度按
/// 8px 色块 + 6px 间隙 + 标签字符宽(label_font_size × 0.6 近似)估算,
/// 超出绘图宽度时截断条目(不换行 — 图表图例惯例)。
pub fn render_legend<T: Theme>(
    theme: &T,
    labels: &[String],
    colors: &[String],
    plot_area: &PlotArea,
) -> RenderOutput {
    let mut output = RenderOutput::new();
    if labels.len() < 2 || labels.len() != colors.len() {
        return output;
    }
    let fs = theme.label_font_size();
    let swatch = 10.0;
    let gap = 6.0;
    let item_gap = 14.0;
    // 从右缘向左排布(与标题共存时互不侵占 — 标题居中,图例贴右)
    let mut x = plot_area.x + plot_area.width;
    let y = plot_area.y - 10.0;
    for (label, color) in labels.iter().zip(colors.iter()).rev() {
        let label_w = label.chars().count() as f64 * fs * 0.6;
        let item_w = swatch + gap + label_w;
        x -= item_w;
        if x < plot_area.x {
            break; // 空间耗尽,丢弃更早的条目
        }
        output.add_command(DrawCmd::Rect {
            x,
            y: y - swatch,
            width: swatch,
            height: swatch,
            fill: Some(FillStyle::Color(color.clone())),
            stroke: None,
            corner_radius: Some(2.0),
            corner_radii: None,
            id: None,
        });
        output.add_command(DrawCmd::Text {
            x: x + swatch + gap,
            y: y - swatch / 2.0,
            content: label.clone(),
            style: TextStyle::new()
                .with_font_size(fs)
                .with_font_family(theme.font_family())
                .with_fill(FillStyle::Color(theme.foreground_color().to_string())),
            anchor: TextAnchor::Start,
            baseline: TextBaseline::Middle,
        });
        x -= item_gap;
    }
    output
}
