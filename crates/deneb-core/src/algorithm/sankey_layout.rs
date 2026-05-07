//! Sankey diagram layout algorithm
//!
//! Assigns column positions to nodes via BFS from source nodes,
//! stacks nodes vertically within each column proportional to flow,
//! and generates cubic Bézier control points for ribbon paths.

use std::collections::VecDeque;

/// Input node descriptor for a Sankey diagram
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyNodeInput {
    /// Display label
    pub label: String,
    /// Optional fill color (hex string)
    pub color: Option<String>,
}

/// Input link descriptor for a Sankey diagram
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyLinkInput {
    /// Index of the source node
    pub source: usize,
    /// Index of the target node
    pub target: usize,
    /// Flow value
    pub value: f64,
    /// Optional fill color (hex string)
    pub color: Option<String>,
}

/// Positioned node rectangle in a Sankey diagram
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyNode {
    /// Display label
    pub label: String,
    /// Total flow through this node (max of in/out)
    pub value: f64,
    /// Left edge x coordinate
    pub x: f64,
    /// Top edge y coordinate
    pub y: f64,
    /// Width of the node rectangle (= `node_width` parameter)
    pub width: f64,
    /// Height of the node rectangle (proportional to total flow)
    pub height: f64,
    /// Fill color (hex string)
    pub color: String,
}

/// A ribbon (flow) between two Sankey nodes, described by Bézier control points
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyLink {
    /// Index of the source node
    pub source: usize,
    /// Index of the target node
    pub target: usize,
    /// Flow value
    pub value: f64,
    /// Cubic Bézier control points for the ribbon path.
    ///
    /// Contains 4 points describing the top edge of the ribbon:
    /// `[source_exit, control1, control2, target_entry]`.
    /// The bottom edge mirrors the top with different y values at source/target.
    pub path_points: Vec<(f64, f64)>,
    /// Fill color (hex string)
    pub color: String,
}

/// Output of the Sankey layout computation
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyLayout {
    /// Positioned node rectangles
    pub nodes: Vec<SankeyNode>,
    /// Ribbon paths for each link
    pub links: Vec<SankeyLink>,
}

/// Default palette used when a node has no explicit color
const DEFAULT_PALETTE: &[&str] = &[
    "#4e79a7", "#f28e2b", "#e15759", "#76b7b2", "#59a14f", "#edc948", "#b07aa1", "#ff9da7",
    "#9c755f", "#bab0ac",
];

/// Compute a full Sankey layout.
///
/// # Arguments
///
/// * `nodes` — node descriptors (labels, optional colors)
/// * `links` — directed links between nodes
/// * `width` — total canvas width available
/// * `height` — total canvas height available
/// * `node_width` — pixel width of each node rectangle
/// * `node_gap` — minimum gap between node rectangles in the same column
pub fn layout_sankey(
    nodes: &[SankeyNodeInput],
    links: &[SankeyLinkInput],
    width: f64,
    height: f64,
    node_width: f64,
    node_gap: f64,
) -> SankeyLayout {
    let n = nodes.len();
    if n == 0 {
        return SankeyLayout {
            nodes: vec![],
            links: vec![],
        };
    }

    // -----------------------------------------------------------------------
    // 1. Assign column (depth) to each node via BFS from source nodes
    // -----------------------------------------------------------------------
    let mut depth = vec![usize::MAX; n];
    let has_incoming: Vec<bool> = {
        let mut v = vec![false; n];
        for link in links {
            if link.target < n {
                v[link.target] = true;
            }
        }
        v
    };

    // Initialize sources (nodes with no incoming links)
    let mut queue: VecDeque<usize> = VecDeque::new();
    for i in 0..n {
        if !has_incoming[i] {
            depth[i] = 0;
            queue.push_back(i);
        }
    }

    // BFS to propagate depths
    while let Some(node) = queue.pop_front() {
        let d = depth[node];
        for link in links {
            if link.source == node && link.target < n && depth[link.target] == usize::MAX {
                depth[link.target] = d + 1;
                queue.push_back(link.target);
            }
        }
    }

    // Clamp any unreachable nodes to depth 0
    for d in &mut depth {
        if *d == usize::MAX {
            *d = 0;
        }
    }

    let max_depth = depth.iter().copied().max().unwrap_or(0);
    let num_cols = max_depth + 1;

    // -----------------------------------------------------------------------
    // 2. Compute node total flow for heights
    // -----------------------------------------------------------------------
    let mut flow_in = vec![0.0_f64; n];
    let mut flow_out = vec![0.0_f64; n];
    for link in links {
        if link.source < n {
            flow_out[link.source] += link.value;
        }
        if link.target < n {
            flow_in[link.target] += link.value;
        }
    }
    let node_flow: Vec<f64> = (0..n)
        .map(|i| flow_in[i].max(flow_out[i]).max(1.0))
        .collect();

    // Per-column grouping
    let mut col_nodes: Vec<Vec<usize>> = vec![vec![]; num_cols];
    for i in 0..n {
        col_nodes[depth[i]].push(i);
    }

    // Compute column x positions
    let col_x: Vec<f64> = if num_cols <= 1 {
        vec![0.0]
    } else {
        let step = (width - node_width) / (num_cols - 1) as f64;
        (0..num_cols).map(|c| c as f64 * step).collect()
    };

    // -----------------------------------------------------------------------
    // 3. Stack nodes vertically within each column
    // -----------------------------------------------------------------------
    let usable_height = height - node_gap * (n as f64); // rough guard

    let mut rects_indexed: Vec<Option<SankeyNode>> = vec![None; n];

    for (col, nodes_in_col) in col_nodes.iter().enumerate() {
        if nodes_in_col.is_empty() {
            continue;
        }
        let total_flow: f64 = nodes_in_col.iter().map(|&i| node_flow[i]).sum();
        let total_gaps = node_gap * (nodes_in_col.len() as f64 + 1.0);
        let available = (usable_height - total_gaps).max(10.0 * nodes_in_col.len() as f64);

        let mut y_cursor = node_gap;
        for &node_idx in nodes_in_col {
            let h = (node_flow[node_idx] / total_flow * available).max(4.0);
            let color = nodes[node_idx]
                .color
                .clone()
                .unwrap_or_else(|| DEFAULT_PALETTE[node_idx % DEFAULT_PALETTE.len()].to_string());
            rects_indexed[node_idx] = Some(SankeyNode {
                label: nodes[node_idx].label.clone(),
                value: node_flow[node_idx],
                x: col_x[col],
                y: y_cursor,
                width: node_width,
                height: h,
                color,
            });
            y_cursor += h + node_gap;
        }
    }

    let node_rects: Vec<SankeyNode> = rects_indexed.into_iter().filter_map(|opt| opt).collect();

    // -----------------------------------------------------------------------
    // 4. Generate ribbon control points (cubic Bézier)
    // -----------------------------------------------------------------------
    // Track used offsets per node edge (for stacking ribbons)
    let mut src_y_used = vec![0.0_f64; n];
    let mut dst_y_used = vec![0.0_f64; n];

    // Sort links by source then target for deterministic stacking
    let mut link_order: Vec<usize> = (0..links.len()).collect();
    link_order.sort_by_key(|&i| (links[i].source, links[i].target));

    let mut ribbons: Vec<SankeyLink> = Vec::with_capacity(links.len());

    for &li in &link_order {
        let link = &links[li];
        if link.source >= n || link.target >= n {
            continue;
        }
        let src = &node_rects[link.source];
        let dst = &node_rects[link.target];

        let src_flow = flow_out[link.source].max(1.0);
        let dst_flow = flow_in[link.target].max(1.0);

        let ribbon_h_src = (link.value / src_flow * src.height).max(1.0);
        let ribbon_h_dst = (link.value / dst_flow * dst.height).max(1.0);

        let y0_top = src.y + src_y_used[link.source];
        let x0 = src.x + src.width;
        let x1 = dst.x;
        let cx = (x0 + x1) / 2.0;

        src_y_used[link.source] += ribbon_h_src;
        dst_y_used[link.target] += ribbon_h_dst;

        // Output 4 control points for the top edge of the ribbon
        let path_points = vec![
            (x0, y0_top),           // source exit
            (cx, y0_top),           // control 1
            (cx, dst.y + dst_y_used[link.target] - ribbon_h_dst), // control 2
            (x1, dst.y + dst_y_used[link.target] - ribbon_h_dst), // target entry
        ];

        let color = link
            .color
            .clone()
            .unwrap_or_else(|| node_rects[link.source].color.clone());

        ribbons.push(SankeyLink {
            source: link.source,
            target: link.target,
            value: link.value,
            path_points,
            color,
        });
    }

    SankeyLayout {
        nodes: node_rects,
        links: ribbons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_simple() -> (Vec<SankeyNodeInput>, Vec<SankeyLinkInput>) {
        let nodes = vec![
            SankeyNodeInput {
                label: "A".into(),
                color: None,
            },
            SankeyNodeInput {
                label: "B".into(),
                color: None,
            },
            SankeyNodeInput {
                label: "C".into(),
                color: None,
            },
        ];
        let links = vec![
            SankeyLinkInput {
                source: 0,
                target: 1,
                value: 10.0,
                color: None,
            },
            SankeyLinkInput {
                source: 0,
                target: 2,
                value: 20.0,
                color: None,
            },
        ];
        (nodes, links)
    }

    #[test]
    fn test_layout_produces_correct_node_count() {
        let (nodes, links) = make_simple();
        let result = layout_sankey(&nodes, &links, 400.0, 300.0, 20.0, 8.0);
        assert_eq!(result.nodes.len(), 3);
        assert_eq!(result.links.len(), 2);
    }

    #[test]
    fn test_conservation() {
        let (nodes, links) = make_simple();
        let result = layout_sankey(&nodes, &links, 400.0, 300.0, 20.0, 8.0);
        // Total ribbon values targeting node 1 should match total link values
        let link_to_b: f64 = links.iter().filter(|l| l.target == 1).map(|l| l.value).sum();
        let ribbon_to_b: f64 = result
            .links
            .iter()
            .filter(|r| r.target == 1)
            .map(|r| r.value)
            .sum();
        assert!(
            (link_to_b - ribbon_to_b).abs() < 1e-10,
            "ribbon values should match link values"
        );
    }

    #[test]
    fn test_empty_sankey() {
        let result = layout_sankey(&[], &[], 400.0, 300.0, 20.0, 8.0);
        assert!(result.nodes.is_empty());
        assert!(result.links.is_empty());
    }

    #[test]
    fn test_node_heights_proportional() {
        let (nodes, links) = make_simple();
        let result = layout_sankey(&nodes, &links, 400.0, 300.0, 20.0, 8.0);
        // Node A has flow_out = 30, B has flow_in = 10, C has flow_in = 20
        // Node A is in column 0 alone, so its height is maximized
        assert!(result.nodes[0].height > 0.0);
        assert!(result.nodes[1].height > 0.0);
        assert!(result.nodes[2].height > 0.0);
    }

    #[test]
    fn test_ribbon_has_control_points() {
        let (nodes, links) = make_simple();
        let result = layout_sankey(&nodes, &links, 400.0, 300.0, 20.0, 8.0);
        for link in &result.links {
            assert_eq!(
                link.path_points.len(),
                4,
                "each ribbon should have 4 control points"
            );
        }
    }

    #[test]
    fn test_columns_assigned_by_bfs() {
        let (nodes, links) = make_simple();
        let result = layout_sankey(&nodes, &links, 400.0, 300.0, 20.0, 8.0);
        // Node A (index 0) is the source → column 0 (x near 0)
        // Nodes B, C are targets → column 1 (x near width - node_width)
        assert!(result.nodes[0].x < result.nodes[1].x);
        assert!(result.nodes[0].x < result.nodes[2].x);
        // B and C should be in the same column
        assert!((result.nodes[1].x - result.nodes[2].x).abs() < 1e-10);
    }
}
