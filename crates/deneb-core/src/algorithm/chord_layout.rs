//! Chord diagram layout algorithm
//!
//! Distributes arcs around a circle proportional to row totals,
//! computes angle ranges for ribbons between groups.

use std::f64::consts::TAU;

/// A chord node (arc segment) representing one group in the chord diagram
#[derive(Debug, Clone, PartialEq)]
pub struct ChordNode {
    /// Index of the group in the original matrix
    pub index: usize,
    /// Start angle in radians (0 = top, clockwise)
    pub start_angle: f64,
    /// End angle in radians
    pub end_angle: f64,
    /// Total value for this group (row total)
    pub value: f64,
}

/// A ribbon between two chord groups, described by angle ranges
#[derive(Debug, Clone, PartialEq)]
pub struct ChordRibbon {
    /// Index of the source group
    pub source: usize,
    /// Index of the target group
    pub target: usize,
    /// Start angle on the source arc for this ribbon
    pub source_start: f64,
    /// End angle on the source arc for this ribbon
    pub source_end: f64,
    /// Start angle on the target arc for this ribbon
    pub target_start: f64,
    /// End angle on the target arc for this ribbon
    pub target_end: f64,
}

/// Complete layout output for a chord diagram
#[derive(Debug, Clone, PartialEq)]
pub struct ChordLayout {
    /// Arc segments (one per group)
    pub nodes: Vec<ChordNode>,
    /// Ribbons (one per non-zero matrix cell)
    pub ribbons: Vec<ChordRibbon>,
}

/// Compute a Chord diagram layout.
///
/// # Arguments
///
/// * `matrix` — square adjacency matrix (`matrix[i][j]` = flow from i to j)
/// * `gap_degrees` — gap in degrees between adjacent arcs
pub fn layout_chord(matrix: &[Vec<f64>], gap_degrees: f64) -> ChordLayout {
    let n = matrix.len();
    if n == 0 {
        return ChordLayout {
            nodes: vec![],
            ribbons: vec![],
        };
    }

    let gap_rad = gap_degrees.to_radians();
    let total_gap = gap_rad * n as f64;
    let available = TAU - total_gap;

    // Row totals drive arc lengths
    let row_totals: Vec<f64> = matrix.iter().map(|row| row.iter().sum::<f64>()).collect();
    let grand_total: f64 = row_totals.iter().sum();
    if grand_total <= 0.0 {
        return ChordLayout {
            nodes: vec![],
            ribbons: vec![],
        };
    }

    // Assign start/end angles for each group arc
    let mut arc_starts = vec![0.0_f64; n];
    let mut arc_ends = vec![0.0_f64; n];
    let mut cursor = 0.0_f64;
    for i in 0..n {
        let span = row_totals[i] / grand_total * available;
        arc_starts[i] = cursor;
        arc_ends[i] = cursor + span;
        cursor += span + gap_rad;
    }

    // Build nodes
    let chord_nodes: Vec<ChordNode> = (0..n)
        .map(|i| ChordNode {
            index: i,
            start_angle: arc_starts[i],
            end_angle: arc_ends[i],
            value: row_totals[i],
        })
        .collect();

    // Build ribbons — track offset within each arc for stacking
    let mut src_offset = vec![0.0_f64; n];
    let mut dst_offset = vec![0.0_f64; n];
    let mut ribbons: Vec<ChordRibbon> = Vec::new();

    for i in 0..n {
        for j in 0..n {
            let v = matrix[i].get(j).copied().unwrap_or(0.0);
            if v <= 0.0 {
                continue;
            }

            let src_span = arc_ends[i] - arc_starts[i];
            let dst_span = arc_ends[j] - arc_starts[j];

            let src_frac = v / grand_total * available / src_span.max(1e-9) * src_span;
            let dst_frac = v / grand_total * available / dst_span.max(1e-9) * dst_span;

            let ss = arc_starts[i] + src_offset[i];
            let se = ss + src_frac.min(src_span - src_offset[i]);
            let ds = arc_starts[j] + dst_offset[j];
            let de = ds + dst_frac.min(dst_span - dst_offset[j]);

            src_offset[i] += src_frac.min(src_span - src_offset[i]);
            dst_offset[j] += dst_frac.min(dst_span - dst_offset[j]);

            ribbons.push(ChordRibbon {
                source: i,
                target: j,
                source_start: ss,
                source_end: se,
                target_start: ds,
                target_end: de,
            });
        }
    }

    ChordLayout {
        nodes: chord_nodes,
        ribbons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    fn make_matrix() -> Vec<Vec<f64>> {
        vec![
            vec![0.0, 10.0, 5.0],
            vec![8.0, 0.0, 12.0],
            vec![3.0, 7.0, 0.0],
        ]
    }

    #[test]
    fn test_node_count() {
        let m = make_matrix();
        let result = layout_chord(&m, 2.0);
        assert_eq!(result.nodes.len(), 3);
    }

    #[test]
    fn test_arc_sum() {
        let m = make_matrix();
        let gap = 2.0_f64;
        let result = layout_chord(&m, gap);
        let total_arc: f64 = result
            .nodes
            .iter()
            .map(|a| a.end_angle - a.start_angle)
            .sum();
        let total_gap = gap.to_radians() * 3.0;
        let expected = TAU - total_gap;
        assert!(
            (total_arc - expected).abs() < 1e-6,
            "arc sum {total_arc:.6} ≠ expected {expected:.6}"
        );
    }

    #[test]
    fn test_empty_chord() {
        let result = layout_chord(&[], 2.0);
        assert!(result.nodes.is_empty());
        assert!(result.ribbons.is_empty());
    }

    #[test]
    fn test_ribbon_values() {
        let m = make_matrix();
        let result = layout_chord(&m, 2.0);
        // Should have ribbons for all non-zero entries (excluding diagonal)
        // m[0][1]=10, m[0][2]=5, m[1][0]=8, m[1][2]=12, m[2][0]=3, m[2][1]=7
        assert_eq!(result.ribbons.len(), 6);
    }

    #[test]
    fn test_node_values_match_row_totals() {
        let m = make_matrix();
        let result = layout_chord(&m, 2.0);
        for (i, node) in result.nodes.iter().enumerate() {
            let expected: f64 = m[i].iter().sum();
            assert!(
                (node.value - expected).abs() < 1e-10,
                "node {} value {} ≠ expected {}",
                i,
                node.value,
                expected
            );
        }
    }

    #[test]
    fn test_ribbon_angles_within_arc_bounds() {
        let m = make_matrix();
        let result = layout_chord(&m, 2.0);
        for ribbon in &result.ribbons {
            let src = &result.nodes[ribbon.source];
            let dst = &result.nodes[ribbon.target];
            // Source ribbon angle should be within source arc
            assert!(
                ribbon.source_start >= src.start_angle - 1e-10,
                "ribbon source_start out of arc bounds"
            );
            assert!(
                ribbon.source_end <= src.end_angle + 1e-10,
                "ribbon source_end out of arc bounds"
            );
            // Target ribbon angle should be within target arc
            assert!(
                ribbon.target_start >= dst.start_angle - 1e-10,
                "ribbon target_start out of arc bounds"
            );
            assert!(
                ribbon.target_end <= dst.end_angle + 1e-10,
                "ribbon target_end out of arc bounds"
            );
        }
    }
}
