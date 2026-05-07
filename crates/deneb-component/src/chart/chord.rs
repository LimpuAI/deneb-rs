//! Chord 图渲染器
//!
//! 将 ChartSpec 和 DataTable 渲染为和弦图的 Canvas 指令。
//! 使用 deneb_core::algorithm::chord_layout 进行弧段和连线的布局计算。

use crate::layout::PlotArea;
use crate::spec::ChartSpec;
use crate::theme::Theme;
use crate::chart::ChartOutput;
use crate::error::ComponentError;
use deneb_core::*;
use deneb_core::algorithm::chord_layout::layout_chord;
use std::collections::HashMap;

/// ChordChart 渲染器
pub struct ChordChart;

/// 弧段之间的间隙角度（度）
const GAP_DEGREES: f64 = 2.0;
/// 弧段内径与外径的比例（厚环形段）
const INNER_RADIUS_RATIO: f64 = 0.85;

impl ChordChart {
    /// 渲染和弦图
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

        // 2. 计算布局区域
        let padding = theme.margin();
        let plot_area = PlotArea {
            x: padding.left,
            y: padding.top,
            width: spec.width - padding.horizontal(),
            height: spec.height - padding.vertical(),
        };

        // 3. 从 DataTable 构建邻接矩阵
        let (categories, matrix) = Self::build_adjacency_matrix(spec, data)?;

        if categories.is_empty() {
            return Ok(Self::render_empty(spec, theme));
        }

        // 单节点 → 单个弧段，无 ribbons
        if categories.len() == 1 {
            let mut layers = RenderLayers::new();
            layers.update_layer(LayerKind::Background, super::shared::render_background_with_border(spec, theme, &plot_area));

            let cx = plot_area.x + plot_area.width / 2.0;
            let cy = plot_area.y + plot_area.height / 2.0;
            let radius = Self::compute_radius(&plot_area);
            let mut data_output = RenderOutput::new();

            // 画完整圆弧
            data_output.add_command(DrawCmd::Arc {
                cx, cy, r: radius,
                start_angle: 0.0,
                end_angle: std::f64::consts::TAU,
                fill: Some(FillStyle::Color(theme.series_color(0).to_string())),
                stroke: Some(StrokeStyle::Color(theme.foreground_color().to_string())),
            });

            // 标签
            let label_style = TextStyle::new()
                .with_font_size(theme.tick_font_size())
                .with_font_family(theme.font_family())
                .with_fill(FillStyle::Color(theme.foreground_color().to_string()));

            data_output.add_command(DrawCmd::Text {
                x: cx,
                y: cy - radius - 10.0,
                content: categories[0].clone(),
                style: label_style,
                anchor: TextAnchor::Middle,
                baseline: TextBaseline::Bottom,
            });

            layers.update_layer(LayerKind::Data, data_output);

            if let Some(title) = &spec.title {
                layers.update_layer(LayerKind::Title, super::shared::render_title(theme, title, &plot_area));
            }

            return Ok(ChartOutput {
                layers,
                hit_regions: Vec::new(),
            });
        }

        // 4. 调用布局算法
        let layout = layout_chord(&matrix, GAP_DEGREES);

        // 5. 生成渲染指令
        let mut layers = RenderLayers::new();
        let mut hit_regions = Vec::new();

        layers.update_layer(LayerKind::Background, super::shared::render_background_with_border(spec, theme, &plot_area));

        let cx = plot_area.x + plot_area.width / 2.0;
        let cy = plot_area.y + plot_area.height / 2.0;
        let radius = Self::compute_radius(&plot_area);

        let (data_commands, regions) = Self::render_chord_elements(
            theme, &layout, &categories, cx, cy, radius,
        );
        layers.update_layer(LayerKind::Data, data_commands);
        hit_regions.extend(regions);

        if let Some(title) = &spec.title {
            layers.update_layer(LayerKind::Title, super::shared::render_title(theme, title, &plot_area));
        }

        Ok(ChartOutput {
            layers,
            hit_regions,
        })
    }

    /// 验证数据
    fn validate_data(spec: &ChartSpec, data: &DataTable) -> Result<(), ComponentError> {
        if spec.encoding.size.is_none() {
            return Err(ComponentError::InvalidConfig {
                reason: "size encoding is required for Chord chart".to_string(),
            });
        }

        if let Some(x_field) = &spec.encoding.x {
            if data.get_column(&x_field.name).is_none() {
                return Err(ComponentError::InvalidConfig {
                    reason: format!("x field '{}' not found in data", x_field.name),
                });
            }
        } else {
            return Err(ComponentError::InvalidConfig {
                reason: "x encoding (source) is required for Chord chart".to_string(),
            });
        }

        if let Some(y_field) = &spec.encoding.y {
            if data.get_column(&y_field.name).is_none() {
                return Err(ComponentError::InvalidConfig {
                    reason: format!("y field '{}' not found in data", y_field.name),
                });
            }
        } else {
            return Err(ComponentError::InvalidConfig {
                reason: "y encoding (target) is required for Chord chart".to_string(),
            });
        }

        if let Some(size_field) = &spec.encoding.size {
            if data.get_column(&size_field.name).is_none() {
                return Err(ComponentError::InvalidConfig {
                    reason: format!("size field '{}' not found in data", size_field.name),
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

    /// 从 DataTable 构建邻接矩阵
    fn build_adjacency_matrix(
        spec: &ChartSpec,
        data: &DataTable,
    ) -> Result<(Vec<String>, Vec<Vec<f64>>), ComponentError> {
        let x_field = spec.encoding.x.as_ref().ok_or_else(|| ComponentError::InvalidConfig {
            reason: "x encoding (source) is required".to_string(),
        })?;
        let y_field = spec.encoding.y.as_ref().ok_or_else(|| ComponentError::InvalidConfig {
            reason: "y encoding (target) is required".to_string(),
        })?;
        let size_field = spec.encoding.size.as_ref().ok_or_else(|| ComponentError::InvalidConfig {
            reason: "size encoding is required".to_string(),
        })?;

        let x_column = data.get_column(&x_field.name).ok_or_else(|| ComponentError::InvalidConfig {
            reason: format!("x field '{}' not found in data", x_field.name),
        })?;
        let y_column = data.get_column(&y_field.name).ok_or_else(|| ComponentError::InvalidConfig {
            reason: format!("y field '{}' not found in data", y_field.name),
        })?;
        let size_column = data.get_column(&size_field.name).ok_or_else(|| ComponentError::InvalidConfig {
            reason: format!("size field '{}' not found in data", size_field.name),
        })?;

        // 收集唯一类别
        let mut category_list: Vec<String> = Vec::new();
        let mut category_index: HashMap<String, usize> = HashMap::new();

        let row_count = data.row_count();
        for row_idx in 0..row_count {
            for col in [x_column, y_column] {
                let name = col
                    .get(row_idx)
                    .and_then(|v| v.as_text())
                    .unwrap_or("")
                    .to_string();
                if !category_index.contains_key(&name) {
                    category_index.insert(name.clone(), category_list.len());
                    category_list.push(name);
                }
            }
        }

        let n = category_list.len();
        let mut matrix = vec![vec![0.0_f64; n]; n];

        for row_idx in 0..row_count {
            let source_name = x_column
                .get(row_idx)
                .and_then(|v| v.as_text())
                .unwrap_or("")
                .to_string();
            let target_name = y_column
                .get(row_idx)
                .and_then(|v| v.as_text())
                .unwrap_or("")
                .to_string();
            let value = size_column
                .get(row_idx)
                .and_then(|v| v.as_numeric())
                .unwrap_or(0.0);

            let si = *category_index.get(&source_name).ok_or_else(|| {
                ComponentError::InvalidConfig {
                    reason: format!("unknown source category: {}", source_name),
                }
            })?;
            let ti = *category_index.get(&target_name).ok_or_else(|| {
                ComponentError::InvalidConfig {
                    reason: format!("unknown target category: {}", target_name),
                }
            })?;
            matrix[si][ti] += value;
        }

        Ok((category_list, matrix))
    }

    /// 计算圆的半径
    fn compute_radius(plot_area: &PlotArea) -> f64 {
        let min_dim = plot_area.width.min(plot_area.height);
        (min_dim / 2.0 - 20.0).max(20.0)
    }

    /// 渲染和弦图元素（弧段、ribbons、标签）
    fn render_chord_elements<T: Theme>(
        theme: &T,
        layout: &deneb_core::algorithm::chord_layout::ChordLayout,
        categories: &[String],
        cx: f64,
        cy: f64,
        radius: f64,
    ) -> (RenderOutput, Vec<HitRegion>) {
        let mut output = RenderOutput::new();
        let mut hit_regions = Vec::new();

        // 先画 ribbons（在弧段之下）
        for ribbon in &layout.ribbons {
            let color = theme.series_color(ribbon.source).to_string();
            let fill_color = Self::with_alpha(&color, 0.4);
            let stroke_color = Self::with_alpha(&color, 0.7);

            // 绘制 ribbon：source 弧段内侧 → 贝塞尔到 target 弧段内侧 → 沿 target 弧 → 贝塞尔回到 source
            let src_start_x = cx + radius * ribbon.source_start.cos();
            let src_start_y = cy + radius * ribbon.source_start.sin();
            let dst_end_x = cx + radius * ribbon.target_end.cos();
            let dst_end_y = cy + radius * ribbon.target_end.sin();

            let mut segments = Vec::new();
            // 从 source 弧起点出发
            segments.push(PathSegment::MoveTo(src_start_x, src_start_y));
            // 沿 source 弧到终点
            segments.push(PathSegment::Arc(cx, cy, radius, ribbon.source_start, ribbon.source_end, false));
            // 二次贝塞尔曲线到 target 弧的终点
            segments.push(PathSegment::QuadraticTo(cx, cy, dst_end_x, dst_end_y));
            // 沿 target 弧（反向）
            segments.push(PathSegment::Arc(cx, cy, radius, ribbon.target_end, ribbon.target_start, true));
            // 二次贝塞尔曲线回到 source 弧的起点
            segments.push(PathSegment::QuadraticTo(cx, cy, src_start_x, src_start_y));
            segments.push(PathSegment::Close);

            output.add_command(DrawCmd::Path {
                segments,
                fill: Some(FillStyle::Color(fill_color)),
                stroke: Some(StrokeStyle::Color(stroke_color)),
            });
        }

        // 画弧段（节点）—— 环形段，不是扇形
        let inner_radius = radius * INNER_RADIUS_RATIO;
        for node in &layout.nodes {
            let color = theme.series_color(node.index).to_string();

            // 环形段路径：外弧 → 连接到内弧终点 → 内弧（反向）→ Close
            let mut segments = Vec::new();
            // 外弧起点
            segments.push(PathSegment::MoveTo(
                cx + radius * node.start_angle.cos(),
                cy + radius * node.start_angle.sin(),
            ));
            // 外弧
            segments.push(PathSegment::Arc(cx, cy, radius, node.start_angle, node.end_angle, false));
            // 连线到内弧终点
            segments.push(PathSegment::LineTo(
                cx + inner_radius * node.end_angle.cos(),
                cy + inner_radius * node.end_angle.sin(),
            ));
            // 内弧（反向）
            segments.push(PathSegment::Arc(cx, cy, inner_radius, node.end_angle, node.start_angle, true));
            segments.push(PathSegment::Close);

            output.add_command(DrawCmd::Path {
                segments,
                fill: Some(FillStyle::Color(color.clone())),
                stroke: Some(StrokeStyle::Color(theme.foreground_color().to_string())),
            });

            // 标签：弧段中点外侧
            let mid_angle = (node.start_angle + node.end_angle) / 2.0;
            let label_radius = radius + 15.0;
            let label_x = cx + label_radius * mid_angle.cos();
            let label_y = cy + label_radius * mid_angle.sin();

            let label_style = TextStyle::new()
                .with_font_size(theme.tick_font_size())
                .with_font_family(theme.font_family())
                .with_fill(FillStyle::Color(theme.foreground_color().to_string()));

            output.add_command(DrawCmd::Text {
                x: label_x,
                y: label_y,
                content: categories.get(node.index)
                    .cloned()
                    .unwrap_or_default(),
                style: label_style,
                anchor: TextAnchor::Middle,
                baseline: TextBaseline::Middle,
            });

            // HitRegion — 用弧段的包围盒近似
            // 计算弧段的包围盒
            let bbox = Self::arc_bounding_box(cx, cy, radius, node.start_angle, node.end_angle);
            let region = HitRegion::new(
                node.index,
                None,
                bbox,
                vec![FieldValue::Text(categories.get(node.index).cloned().unwrap_or_default())],
            );
            hit_regions.push(region);
        }

        (output, hit_regions)
    }

    /// 计算弧段的包围盒
    fn arc_bounding_box(cx: f64, cy: f64, r: f64, start: f64, end: f64) -> BoundingBox {
        // 检查弧段是否经过每个象限的边界角度
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        // 采样弧段上的点
        let steps = 20;
        for i in 0..=steps {
            let angle = start + (end - start) * i as f64 / steps as f64;
            let x = cx + r * angle.cos();
            let y = cy + r * angle.sin();
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }

        BoundingBox::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// 为颜色添加透明度
    fn with_alpha(color: &str, alpha: f64) -> String {
        let hex = color.trim_start_matches('#');
        if hex.len() == 6 {
            format!("#{}{:02x}", hex, (alpha * 255.0) as u8)
        } else {
            color.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{Encoding, Mark, Field};
    use crate::theme::DefaultTheme;

    fn create_chord_data() -> DataTable {
        DataTable::with_columns(vec![
            Column::new("source", DataType::Nominal, vec![
                FieldValue::Text("A".to_string()),
                FieldValue::Text("A".to_string()),
                FieldValue::Text("B".to_string()),
                FieldValue::Text("B".to_string()),
                FieldValue::Text("C".to_string()),
                FieldValue::Text("C".to_string()),
            ]),
            Column::new("target", DataType::Nominal, vec![
                FieldValue::Text("B".to_string()),
                FieldValue::Text("C".to_string()),
                FieldValue::Text("A".to_string()),
                FieldValue::Text("C".to_string()),
                FieldValue::Text("A".to_string()),
                FieldValue::Text("B".to_string()),
            ]),
            Column::new("value", DataType::Quantitative, vec![
                FieldValue::Numeric(10.0),
                FieldValue::Numeric(5.0),
                FieldValue::Numeric(8.0),
                FieldValue::Numeric(12.0),
                FieldValue::Numeric(3.0),
                FieldValue::Numeric(7.0),
            ]),
        ])
    }

    fn create_chord_spec() -> ChartSpec {
        ChartSpec::builder()
            .mark(Mark::Chord)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("source"))
                    .y(Field::nominal("target"))
                    .size(Field::quantitative("value")),
            )
            .width(400.0)
            .height(400.0)
            .build()
            .unwrap()
    }

    #[test]
    fn test_chord_render_basic() {
        let spec = create_chord_spec();
        let theme = DefaultTheme;
        let data = create_chord_data();

        let result = ChordChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        // 3 categories → 3 arc nodes
        assert_eq!(output.hit_regions.len(), 3);
        assert!(output.layers.get_layer(LayerKind::Background).is_some());
        assert!(output.layers.get_layer(LayerKind::Data).is_some());
    }

    #[test]
    fn test_chord_render_empty() {
        let spec = create_chord_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("source", DataType::Nominal, vec![]),
            Column::new("target", DataType::Nominal, vec![]),
            Column::new("value", DataType::Quantitative, vec![]),
        ]);

        let result = ChordChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.hit_regions.is_empty());
    }

    #[test]
    fn test_chord_render_single_node() {
        let spec = create_chord_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("source", DataType::Nominal, vec![
                FieldValue::Text("A".to_string()),
            ]),
            Column::new("target", DataType::Nominal, vec![
                FieldValue::Text("A".to_string()),
            ]),
            Column::new("value", DataType::Quantitative, vec![
                FieldValue::Numeric(10.0),
            ]),
        ]);

        let result = ChordChart::render(&spec, &theme, &data);
        assert!(result.is_ok());
        // 单节点 → 无 hit_regions（只有弧段）
        let output = result.unwrap();
        assert!(output.hit_regions.is_empty());
    }

    #[test]
    fn test_chord_validate_missing_size() {
        let spec = ChartSpec::builder()
            .mark(Mark::Chord)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("source"))
                    .y(Field::nominal("target")),
            )
            .width(400.0)
            .height(400.0)
            .build()
            .unwrap();

        let theme = DefaultTheme;
        let data = create_chord_data();

        let result = ChordChart::render(&spec, &theme, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("size"));
    }

    #[test]
    fn test_chord_with_title() {
        let spec = ChartSpec::builder()
            .mark(Mark::Chord)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("source"))
                    .y(Field::nominal("target"))
                    .size(Field::quantitative("value")),
            )
            .title("Migration Flow")
            .width(400.0)
            .height(400.0)
            .build()
            .unwrap();

        let theme = DefaultTheme;
        let data = create_chord_data();

        let result = ChordChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        let title_layer = output.layers.get_layer(LayerKind::Title);
        assert!(title_layer.is_some());
    }
}
