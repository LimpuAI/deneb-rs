//! Sankey 图渲染器
//!
//! 将 ChartSpec 和 DataTable 渲染为桑基图的 Canvas 指令。
//! 使用 deneb_core::algorithm::sankey_layout 进行节点和连线的布局计算。

use crate::layout::PlotArea;
use crate::spec::ChartSpec;
use crate::theme::Theme;
use crate::chart::ChartOutput;
use crate::error::ComponentError;
use deneb_core::*;
use deneb_core::algorithm::sankey_layout::{
    layout_sankey, SankeyNodeInput, SankeyLinkInput,
};
use std::collections::HashMap;

/// SankeyChart 渲染器
pub struct SankeyChart;

/// Sankey 图节点宽度（像素）
const NODE_WIDTH: f64 = 20.0;
/// Sankey 图节点最小间距（像素）
const NODE_GAP: f64 = 8.0;

impl SankeyChart {
    /// 渲染桑基图
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

        // 3. 从 DataTable 提取节点和连线
        let (nodes, links, node_index_map) = Self::extract_nodes_and_links(spec, data)?;

        if nodes.is_empty() {
            return Ok(Self::render_empty(spec, theme));
        }

        // 4. 调用布局算法
        let layout = layout_sankey(
            &nodes,
            &links,
            plot_area.width,
            plot_area.height,
            NODE_WIDTH,
            NODE_GAP,
        );

        // 5. 生成渲染指令
        let mut layers = RenderLayers::new();
        let mut hit_regions = Vec::new();

        // 背景层
        layers.update_layer(LayerKind::Background, super::shared::render_background_with_border(spec, theme, &plot_area));

        // 数据层（节点和连线）
        let (data_commands, regions) = Self::render_sankey_elements(
            theme, &layout, &node_index_map, &plot_area, data,
        );
        layers.update_layer(LayerKind::Data, data_commands);
        hit_regions.extend(regions);

        // 标题层
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
        // 检查 encoding.size 是否存在
        if spec.encoding.size.is_none() {
            return Err(ComponentError::InvalidConfig {
                reason: "size encoding is required for Sankey chart".to_string(),
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
                reason: "x encoding (source) is required for Sankey chart".to_string(),
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
                reason: "y encoding (target) is required for Sankey chart".to_string(),
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

    /// 从 DataTable 提取唯一节点和连线
    fn extract_nodes_and_links(
        spec: &ChartSpec,
        data: &DataTable,
    ) -> Result<(Vec<SankeyNodeInput>, Vec<SankeyLinkInput>, HashMap<String, usize>), ComponentError> {
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

        let color_column = spec.encoding.color.as_ref()
            .and_then(|field| data.get_column(&field.name));

        // 收集唯一节点名
        let mut node_names: Vec<String> = Vec::new();
        let mut node_index_map: HashMap<String, usize> = HashMap::new();

        let row_count = data.row_count();
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

            for name in [&source_name, &target_name] {
                if !node_index_map.contains_key(name) {
                    node_index_map.insert(name.clone(), node_names.len());
                    node_names.push(name.clone());
                }
            }
        }

        // 构建节点输入
        let nodes: Vec<SankeyNodeInput> = node_names.iter().map(|name| {
            SankeyNodeInput {
                label: name.clone(),
                color: None,
            }
        }).collect();

        // 构建连线输入
        let mut links: Vec<SankeyLinkInput> = Vec::new();
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

            let link_color = color_column
                .and_then(|col| col.get(row_idx))
                .and_then(|v| v.as_text())
                .map(|s| s.to_string());

            // 零 flow 跳过 ribbon，但保留节点
            if value <= 0.0 {
                continue;
            }

            let source_idx = *node_index_map.get(&source_name).ok_or_else(|| {
                ComponentError::InvalidConfig {
                    reason: format!("unknown source node: {}", source_name),
                }
            })?;
            let target_idx = *node_index_map.get(&target_name).ok_or_else(|| {
                ComponentError::InvalidConfig {
                    reason: format!("unknown target node: {}", target_name),
                }
            })?;

            links.push(SankeyLinkInput {
                source: source_idx,
                target: target_idx,
                value,
                color: link_color,
            });
        }

        Ok((nodes, links, node_index_map))
    }

    /// 渲染桑基图元素（节点、连线、标签）
    fn render_sankey_elements<T: Theme>(
        theme: &T,
        layout: &deneb_core::algorithm::sankey_layout::SankeyLayout,
        _node_index_map: &HashMap<String, usize>,
        plot_area: &PlotArea,
        _data: &DataTable,
    ) -> (RenderOutput, Vec<HitRegion>) {
        let mut output = RenderOutput::new();
        let mut hit_regions = Vec::new();

        // 渲染连线（ribbon）—— 先画连线再画节点，确保节点在上层
        for link in &layout.links {
            if link.path_points.len() < 4 {
                continue;
            }

            let (x0, y0_top) = link.path_points[0];
            let (cx1, cy1) = link.path_points[1];
            let (cx2, cy2) = link.path_points[2];
            let (x1, y1_top) = link.path_points[3];

            // 需要计算 ribbon 的底部 y 值
            // 使用 source 和 target 节点的信息计算 ribbon 高度
            let source_node = &layout.nodes[link.source];
            let target_node = &layout.nodes[link.target];
            let src_flow = source_node.value.max(1.0);
            let dst_flow = target_node.value.max(1.0);
            let ribbon_h_src = (link.value / src_flow * source_node.height).max(1.0);
            let ribbon_h_dst = (link.value / dst_flow * target_node.height).max(1.0);

            let y0_bot = y0_top + ribbon_h_src;
            let y1_bot = y1_top + ribbon_h_dst;

            // 绘制 ribbon 路径：顶部边 → 右侧 → 底部边（反向）→ 左侧
            let mut segments = Vec::new();
            segments.push(PathSegment::MoveTo(x0, y0_top));
            segments.push(PathSegment::BezierTo(cx1, cy1, cx2, cy2, x1, y1_top));
            segments.push(PathSegment::LineTo(x1, y1_bot));
            segments.push(PathSegment::BezierTo(cx2, cy2, cx1, cy1, x0, y0_bot));
            segments.push(PathSegment::Close);

            output.add_command(DrawCmd::Path {
                segments,
                fill: Some(FillStyle::Color(Self::with_alpha(&link.color, 0.5))),
                stroke: Some(StrokeStyle::Color(Self::with_alpha(&link.color, 0.8))),
            });
        }

        // 渲染节点
        for (idx, node) in layout.nodes.iter().enumerate() {
            let node_x = plot_area.x + node.x;
            let node_y = plot_area.y + node.y;

            output.add_command(DrawCmd::Rect {
                x: node_x,
                y: node_y,
                width: node.width,
                height: node.height,
                fill: Some(FillStyle::Color(node.color.clone())),
                stroke: None,
                corner_radius: Some(2.0),
            });

            // 节点标签
            let label_style = TextStyle::new()
                .with_font_size(theme.tick_font_size())
                .with_font_family(theme.font_family())
                .with_fill(FillStyle::Color(theme.foreground_color().to_string()));

            // 标签位置：节点右侧或左侧
            let (label_x, anchor) = {
                // 如果节点在右半部分，标签放右侧；否则放左侧
                let node_center_x = node_x + node.width / 2.0;
                if node_center_x > plot_area.x + plot_area.width / 2.0 {
                    (node_x + node.width + 4.0, TextAnchor::Start)
                } else {
                    (node_x - 4.0, TextAnchor::End)
                }
            };

            output.add_command(DrawCmd::Text {
                x: label_x,
                y: node_y + node.height / 2.0,
                content: node.label.clone(),
                style: label_style,
                anchor,
                baseline: TextBaseline::Middle,
            });

            // 创建 HitRegion
            let region = HitRegion::from_rect(
                node_x,
                node_y,
                node.width,
                node.height,
                idx,
                None,
                vec![FieldValue::Text(node.label.clone()), FieldValue::Numeric(node.value)],
            );
            hit_regions.push(region);
        }

        (output, hit_regions)
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

    fn create_sankey_data() -> DataTable {
        DataTable::with_columns(vec![
            Column::new("source", DataType::Nominal, vec![
                FieldValue::Text("A".to_string()),
                FieldValue::Text("A".to_string()),
                FieldValue::Text("B".to_string()),
            ]),
            Column::new("target", DataType::Nominal, vec![
                FieldValue::Text("B".to_string()),
                FieldValue::Text("C".to_string()),
                FieldValue::Text("C".to_string()),
            ]),
            Column::new("value", DataType::Quantitative, vec![
                FieldValue::Numeric(10.0),
                FieldValue::Numeric(20.0),
                FieldValue::Numeric(15.0),
            ]),
        ])
    }

    fn create_sankey_spec() -> ChartSpec {
        ChartSpec::builder()
            .mark(Mark::Sankey)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("source"))
                    .y(Field::nominal("target"))
                    .size(Field::quantitative("value")),
            )
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap()
    }

    #[test]
    fn test_sankey_render_basic() {
        let spec = create_sankey_spec();
        let theme = DefaultTheme;
        let data = create_sankey_data();

        let result = SankeyChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        // 3 nodes → 3 hit regions
        assert_eq!(output.hit_regions.len(), 3);
        assert!(output.layers.get_layer(LayerKind::Background).is_some());
        assert!(output.layers.get_layer(LayerKind::Data).is_some());
    }

    #[test]
    fn test_sankey_render_empty() {
        let spec = create_sankey_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("source", DataType::Nominal, vec![]),
            Column::new("target", DataType::Nominal, vec![]),
            Column::new("value", DataType::Quantitative, vec![]),
        ]);

        let result = SankeyChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.hit_regions.is_empty());
    }

    #[test]
    fn test_sankey_render_with_title() {
        let spec = ChartSpec::builder()
            .mark(Mark::Sankey)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("source"))
                    .y(Field::nominal("target"))
                    .size(Field::quantitative("value")),
            )
            .title("Energy Flow")
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap();

        let theme = DefaultTheme;
        let data = create_sankey_data();

        let result = SankeyChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        let title_layer = output.layers.get_layer(LayerKind::Title);
        assert!(title_layer.is_some());
        assert!(!title_layer.unwrap().commands.is_empty());
    }

    #[test]
    fn test_sankey_validate_missing_size() {
        let spec = ChartSpec::builder()
            .mark(Mark::Sankey)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("source"))
                    .y(Field::nominal("target")),
            )
            .width(400.0)
            .height(300.0)
            .build()
            .unwrap();

        let theme = DefaultTheme;
        let data = create_sankey_data();

        let result = SankeyChart::render(&spec, &theme, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("size"));
    }

    #[test]
    fn test_sankey_validate_missing_field() {
        let spec = create_sankey_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("wrong", DataType::Nominal, vec![]),
        ]);

        let result = SankeyChart::render(&spec, &theme, &data);
        assert!(result.is_err());
    }

    #[test]
    fn test_sankey_zero_flow_keeps_nodes() {
        // 零 flow 的连线应跳过，但节点仍保留
        let spec = create_sankey_spec();
        let theme = DefaultTheme;
        let data = DataTable::with_columns(vec![
            Column::new("source", DataType::Nominal, vec![
                FieldValue::Text("A".to_string()),
                FieldValue::Text("A".to_string()),
            ]),
            Column::new("target", DataType::Nominal, vec![
                FieldValue::Text("B".to_string()),
                FieldValue::Text("C".to_string()),
            ]),
            Column::new("value", DataType::Quantitative, vec![
                FieldValue::Numeric(0.0),
                FieldValue::Numeric(10.0),
            ]),
        ]);

        let result = SankeyChart::render(&spec, &theme, &data);
        assert!(result.is_ok());

        let output = result.unwrap();
        // 3 个节点（A, B, C）都应存在
        assert_eq!(output.hit_regions.len(), 3);
    }
}
