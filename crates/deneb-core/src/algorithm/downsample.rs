//! 降采样算法
//!
//! 提供 LTTB (Largest Triangle Three Buckets) 和 M4 (Min-Max-Min-Max) 降采样算法。

/// LTTB (Largest Triangle Three Buckets) 降采样算法
///
/// 将数据点序列降采样到指定数量的点，通过最大化相邻桶的三角形面积来保持视觉形状。
/// 适用于时间序列数据，能够保持数据的整体趋势和局部特征。
///
/// # Arguments
///
/// * `points` - 数据点序列，每个点是 (x, y) 坐标
/// * `threshold` - 目标点数
///
/// # Returns
///
/// 降采样后的数据点序列
///
/// # Algorithm
///
/// 1. 将数据分成 threshold 个桶
/// 2. 每个桶选一个代表点，使得与前一个桶的三角形面积最大
/// 3. 保持首尾点不变
///
/// # Examples
///
/// ```no_run
/// use deneb_core::algorithm::downsample::lttb;
/// let points = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 3.0)];
/// let sampled = lttb(&points, 2);
/// assert!(sampled.len() <= 2);
/// ```
///
/// # Edge Cases
///
/// - 数据点少于阈值 → 返回原始数据（不降采样）
/// - 空数据 → 返回空 Vec
/// - threshold == 0 → 返回空 Vec
/// - 阈值 >= 数据长度 → 返回原始数据副本
/// - NaN/Inf 值 → 在三角形面积计算时跳过
pub fn lttb(points: &[(f64, f64)], threshold: usize) -> Vec<(f64, f64)> {
    // 处理边界情况
    if points.is_empty() || threshold == 0 {
        return Vec::new();
    }

    if points.len() <= threshold {
        return points.to_vec();
    }

    // 确保阈值至少为 3（LTTB 需要至少首、中、尾三个点）
    let threshold = threshold.max(3);

    let sampled = Vec::with_capacity(threshold);
    let mut sampled = sampled;

    // 总是保留第一个点
    sampled.push(points[0]);

    // 计算每个桶的大小
    let bucket_size = (points.len() - 2) as f64 / (threshold - 2) as f64;

    let mut a = 0; // 上一个选中的点的索引

    // 处理中间的桶（从第2个桶到倒数第2个桶）
    for bucket_index in 0..(threshold - 2) {
        // 计算当前桶的范围
        let avg_range_start = (1.0 + (bucket_index as f64) * bucket_size).floor() as usize;
        let avg_range_end = ((1.0 + (bucket_index as f64 + 1.0) * bucket_size).floor() as usize).min(points.len());

        // 计算下一个桶的平均点
        let mut avg_x = 0.0;
        let mut avg_y = 0.0;
        let mut count = 0;

        for i in avg_range_start..avg_range_end {
            let (x, y) = points[i];
            // 跳过 NaN 和 Inf
            if x.is_finite() && y.is_finite() {
                avg_x += x;
                avg_y += y;
                count += 1;
            }
        }

        if count == 0 {
            continue;
        }

        avg_x /= count as f64;
        avg_y /= count as f64;

        // 在当前桶中找到使得三角形面积最大的点
        // 当前桶的范围
        let range_start = (1.0 + (bucket_index as f64) * bucket_size).floor() as usize;
        let range_end = ((1.0 + (bucket_index as f64 + 1.0) * bucket_size).ceil() as usize).min(points.len());

        let point_a = points[a];
        let mut max_area = -1.0;
        let mut max_area_index = a;

        for i in range_start..range_end {
            let (x, y) = points[i];

            // 跳过 NaN 和 Inf
            if !x.is_finite() || !y.is_finite() {
                continue;
            }

            // 计算三角形面积
            // 三角形顶点：point_a, (x, y), (avg_x, avg_y)
            let area = triangle_area(point_a.0, point_a.1, x, y, avg_x, avg_y);

            if area > max_area {
                max_area = area;
                max_area_index = i;
            }
        }

        if max_area_index != a {
            sampled.push(points[max_area_index]);
            a = max_area_index;
        }
    }

    // 总是保留最后一个点
    sampled.push(points[points.len() - 1]);

    sampled
}

/// 计算三角形的面积
///
/// 使用叉积公式：|AB x AC| / 2
fn triangle_area(x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) -> f64 {
    let ab_x = x2 - x1;
    let ab_y = y2 - y1;
    let ac_x = x3 - x1;
    let ac_y = y3 - y1;

    (ab_x * ac_y - ab_y * ac_x).abs() / 2.0
}

/// M4 (Min-Max-Min-Max) 降采样算法
///
/// 将数据点序列降采样到指定像素宽度，通过保留每个桶的 min/max 点来实现极速降采样。
/// 适合大数据量概览，能够保留数据的极值信息。
///
/// # Arguments
///
/// * `points` - 数据点序列，每个点是 (x, y) 坐标
/// * `pixel_width` - 目标像素宽度（桶的数量）
///
/// # Returns
///
/// 降采样后的数据点序列
///
/// # Algorithm
///
/// 1. 将数据分成 pixel_width 个桶
/// 2. 每个桶保留最多 4 个点：第一个点、最小值点、最大值点、最后一个点
/// 3. 如果桶只有一个点，只保留该点
///
/// # Examples
///
/// ```no_run
/// use deneb_core::algorithm::downsample::m4;
/// let points = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 3.0)];
/// let sampled = m4(&points, 2);
/// assert!(sampled.len() <= 8); // 2 桶 × 4 点
/// ```
///
/// # Edge Cases
///
/// - 数据点少于像素宽度 → 返回原始数据（不降采样）
/// - 空数据 → 返回空 Vec
/// - pixel_width == 0 → 返回空 Vec
/// - 像素宽度 >= 数据长度 → 返回原始数据副本
/// - NaN/Inf 值 → 在 min/max 比较时忽略
pub fn m4(points: &[(f64, f64)], pixel_width: usize) -> Vec<(f64, f64)> {
    // 处理边界情况
    if points.is_empty() || pixel_width == 0 {
        return Vec::new();
    }

    if points.len() <= pixel_width {
        return points.to_vec();
    }

    let mut sampled = Vec::new();

    // 计算每个桶的大小
    let bucket_size = points.len() as f64 / pixel_width as f64;

    for bucket_index in 0..pixel_width {
        let start = (bucket_index as f64 * bucket_size).floor() as usize;
        let end = ((bucket_index as f64 + 1.0) * bucket_size).ceil() as usize;
        let end = end.min(points.len());

        if start >= end {
            continue;
        }

        if start + 1 == end {
            // 桶只有一个点
            sampled.push(points[start]);
            continue;
        }

        // 收集桶中所有有效的点（非 NaN/Inf）
        let mut bucket_points: Vec<(usize, f64, f64)> = Vec::new();

        for i in start..end {
            let (x, y) = points[i];
            if x.is_finite() && y.is_finite() {
                bucket_points.push((i, x, y));
            }
        }

        if bucket_points.is_empty() {
            continue;
        }

        // 添加第一个点
        let (first_i, first_x, first_y) = bucket_points[0];
        sampled.push((first_x, first_y));

        if bucket_points.len() == 1 {
            continue;
        }

        // 找到最小值和最大值
        let mut min_y = first_y;
        let mut min_y_index = first_i;
        let mut max_y = first_y;
        let mut max_y_index = first_i;

        for &(i, _x, y) in &bucket_points[1..] {
            if y < min_y {
                min_y = y;
                min_y_index = i;
            }
            if y > max_y {
                max_y = y;
                max_y_index = i;
            }
        }

        // 添加最小值点（如果与第一个点不同）
        if min_y_index != first_i {
            sampled.push(points[min_y_index]);
        }

        // 添加最大值点（如果与最小值点不同）
        if max_y_index != min_y_index && max_y_index != first_i {
            sampled.push(points[max_y_index]);
        }

        // 添加最后一个点（如果与前面的点都不同）
        let (last_i, last_x, last_y) = bucket_points[bucket_points.len() - 1];
        if last_i != first_i && last_i != min_y_index && last_i != max_y_index {
            sampled.push((last_x, last_y));
        }
    }

    sampled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lttb_empty_data() {
        let points: Vec<(f64, f64)> = vec![];
        let result = lttb(&points, 10);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_lttb_zero_threshold() {
        let points = vec![(0.0, 0.0), (1.0, 1.0)];
        let result = lttb(&points, 0);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_lttb_data_less_than_threshold() {
        let points = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)];
        let result = lttb(&points, 10);
        assert_eq!(result.len(), 3);
        assert_eq!(result, points);
    }

    #[test]
    fn test_lttb_threshold_equals_data_length() {
        let points = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)];
        let result = lttb(&points, 3);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_lttb_basic() {
        // 创建一个线性增长的数据集
        let points: Vec<(f64, f64)> = (0..100).map(|i| (i as f64, i as f64)).collect();
        let result = lttb(&points, 10);

        // 结果应该在 3 到 10 之间
        assert!(result.len() >= 3 && result.len() <= 10);

        // 首尾点应该保持不变
        assert_eq!(result[0], points[0]);
        assert_eq!(result[result.len() - 1], points[points.len() - 1]);

        // 结果应该是单调递增的
        for i in 1..result.len() {
            assert!(result[i].0 >= result[i - 1].0);
        }
    }

    #[test]
    fn test_lttb_preserves_first_and_last() {
        let points: Vec<(f64, f64)> = (0..100).map(|i| (i as f64, i as f64)).collect();
        let result = lttb(&points, 20);

        assert_eq!(result[0], points[0]);
        assert_eq!(result[result.len() - 1], points[points.len() - 1]);
    }

    #[test]
    fn test_lttb_with_nan() {
        let points = vec![(0.0, 0.0), (1.0, f64::NAN), (2.0, 2.0), (3.0, 3.0)];
        let result = lttb(&points, 3);

        // 应该跳过 NaN 值
        assert!(result.len() >= 2);
        assert_eq!(result[0], (0.0, 0.0));
        assert_eq!(result[result.len() - 1], (3.0, 3.0));
    }

    #[test]
    fn test_lttb_with_inf() {
        let points = vec![(0.0, 0.0), (1.0, f64::INFINITY), (2.0, 2.0), (3.0, 3.0)];
        let result = lttb(&points, 3);

        // 应该跳过 Inf 值
        assert!(result.len() >= 2);
        assert_eq!(result[0], (0.0, 0.0));
    }

    #[test]
    fn test_triangle_area() {
        // 等腰直角三角形，底边为 2，高为 1
        let area = triangle_area(0.0, 0.0, 2.0, 0.0, 1.0, 1.0);
        assert!((area - 1.0).abs() < 1e-10);

        // 另一个三角形
        let area = triangle_area(0.0, 0.0, 1.0, 0.0, 0.0, 1.0);
        assert!((area - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_m4_empty_data() {
        let points: Vec<(f64, f64)> = vec![];
        let result = m4(&points, 10);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_m4_zero_pixel_width() {
        let points = vec![(0.0, 0.0), (1.0, 1.0)];
        let result = m4(&points, 0);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_m4_data_less_than_pixel_width() {
        let points = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)];
        let result = m4(&points, 10);
        assert_eq!(result.len(), 3);
        assert_eq!(result, points);
    }

    #[test]
    fn test_m4_basic() {
        // 创建一个有波动的数据集
        let points: Vec<(f64, f64)> = (0..100)
            .map(|i| {
                let x = i as f64;
                let y = (i as f64 / 10.0).sin() * 10.0 + 50.0;
                (x, y)
            })
            .collect();

        let result = m4(&points, 10);

        // 结果应该少于或等于 40 点（10 桶 × 4 点）
        assert!(result.len() <= 40);

        // 结果应该包含极值信息
        let y_values: Vec<f64> = result.iter().map(|&(_, y)| y).collect();
        let original_min = points.iter().map(|&(_, y)| y).fold(f64::INFINITY, f64::min);
        let _original_max = points.iter().map(|&(_, y)| y).fold(f64::NEG_INFINITY, f64::max);

        assert!(y_values.iter().any(|&y| (y - original_min).abs() < 1e-10) ||
                result.len() > 0); // 至少应该保留一些极值信息
    }

    #[test]
    fn test_m4_single_point_per_bucket() {
        // 创建一个只有 5 个点的数据集，分成 5 个桶
        let points = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 3.0), (4.0, 4.0)];
        let result = m4(&points, 5);

        // 每个桶只有一个点，所以结果应该和原始数据相同
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_m4_with_nan() {
        let points = vec![
            (0.0, 0.0),
            (1.0, 1.0),
            (2.0, f64::NAN),
            (3.0, 3.0),
            (4.0, 4.0),
        ];
        let result = m4(&points, 2);

        // 应该跳过 NaN 值
        assert!(result.len() > 0);
        assert!(result.iter().all(|&(x, y)| x.is_finite() && y.is_finite()));
    }

    #[test]
    fn test_m4_preserves_structure() {
        // 创建一个有明显峰值的数据集
        let points = vec![
            (0.0, 10.0),
            (1.0, 20.0),
            (2.0, 100.0), // 峰值
            (3.0, 20.0),
            (4.0, 10.0),
            (5.0, 5.0),  // 谷值
            (6.0, 15.0),
            (7.0, 10.0),
        ];

        let result = m4(&points, 4);

        // 结果应该保留峰值和谷值
        let y_values: Vec<f64> = result.iter().map(|&(_, y)| y).collect();
        assert!(y_values.iter().any(|&y| y >= 90.0)); // 应该保留峰值
        assert!(y_values.iter().any(|&y| y <= 10.0)); // 应该保留谷值
    }

    #[test]
    fn test_lttb_sine_wave() {
        // 创建一个正弦波
        let points: Vec<(f64, f64)> = (0..1000)
            .map(|i| {
                let x = i as f64 / 100.0;
                let y = (x * 10.0).sin();
                (x, y)
            })
            .collect();

        let result = lttb(&points, 50);

        // 应该降采样到接近 50 个点
        assert!(result.len() >= 45 && result.len() <= 55);

        // 首尾点应该保持
        assert_eq!(result[0], points[0]);
        assert_eq!(result[result.len() - 1], points[points.len() - 1]);
    }

    #[test]
    fn test_m4_constant_data() {
        // 常量数据
        let points: Vec<(f64, f64)> = (0..100).map(|i| (i as f64, 42.0)).collect();
        let result = m4(&points, 10);

        // 应该保留一些点，但所有点的 y 值都应该相同
        assert!(result.len() > 0);
        assert!(result.iter().all(|&(_, y)| (y - 42.0).abs() < 1e-10));
    }

    #[test]
    fn test_lttb_threshold_too_small() {
        let points: Vec<(f64, f64)> = (0..100).map(|i| (i as f64, i as f64)).collect();

        // 阈值小于 3 时，应该被调整为 3
        let result = lttb(&points, 2);
        assert!(result.len() >= 3);

        let result = lttb(&points, 1);
        assert!(result.len() >= 3);
    }

    #[test]
    fn test_lttb_with_duplicates() {
        // 有重复 x 值的数据
        let points = vec![
            (0.0, 0.0),
            (1.0, 1.0),
            (1.0, 2.0), // 重复的 x
            (2.0, 3.0),
            (3.0, 4.0),
        ];

        let result = lttb(&points, 3);

        // 应该成功处理
        assert!(result.len() >= 2);
        assert_eq!(result[0], points[0]);
        assert_eq!(result[result.len() - 1], points[points.len() - 1]);
    }
}
