//! 坐标反查模块
//!
//! 提供像素坐标到数据空间的反向映射能力。

use crate::data::FieldValue;
use crate::interaction::{HitResult, HitRegion};
use crate::scale::{LinearScale, Scale};

/// 坐标反查接口 — 像素坐标到数据空间的反向映射
///
/// 提供命中测试和坐标反转的抽象接口。
pub trait CoordLookup {
    /// 像素坐标 → 最近的数据点命中结果
    ///
    /// # Arguments
    ///
    /// * `x` - 像素 x 坐标
    /// * `y` - 像素 y 坐标
    /// * `tolerance` - 容差范围，扩展命中区域的搜索范围
    ///
    /// # Returns
    ///
    /// 如果找到命中点，返回 Some(HitResult)，否则返回 None
    fn hit_test(&self, x: f64, y: f64, tolerance: f64) -> Option<HitResult>;

    /// 像素坐标 → 数据空间值
    ///
    /// # Arguments
    ///
    /// * `x` - 像素 x 坐标
    /// * `y` - 像素 y 坐标
    ///
    /// # Returns
    ///
    /// 返回 (x_data, y_data) 的数据值，如果转换失败返回 None
    fn invert(&self, x: f64, y: f64) -> Option<(FieldValue, FieldValue)>;
}

/// 简单的线性扫描命中检测 — 适用于 <100K 数据点
///
/// 通过遍历所有命中区域进行线性查找，适用于中小规模数据集。
pub struct SimpleLookup {
    /// 命中区域列表
    regions: Vec<HitRegion>,
    /// x 轴 Scale（用于 invert）
    x_scale: LinearScale,
    /// y 轴 Scale（用于 invert）
    y_scale: LinearScale,
}

impl SimpleLookup {
    /// 创建新的简单查找器
    ///
    /// # Arguments
    ///
    /// * `regions` - 命中区域列表
    /// * `x_scale` - x 轴比例尺
    /// * `y_scale` - y 轴比例尺
    pub fn new(regions: Vec<HitRegion>, x_scale: LinearScale, y_scale: LinearScale) -> Self {
        Self {
            regions,
            x_scale,
            y_scale,
        }
    }

    /// 获取命中区域列表
    pub fn regions(&self) -> &[HitRegion] {
        &self.regions
    }

    /// 获取 x 轴比例尺
    pub fn x_scale(&self) -> &LinearScale {
        &self.x_scale
    }

    /// 获取 y 轴比例尺
    pub fn y_scale(&self) -> &LinearScale {
        &self.y_scale
    }
}

impl CoordLookup for SimpleLookup {
    fn hit_test(&self, x: f64, y: f64, tolerance: f64) -> Option<HitResult> {
        let mut closest: Option<(usize, f64, &HitRegion)> = None;

        for (idx, region) in self.regions.iter().enumerate() {
            // 使用 tolerance 扩展包围盒
            let expanded_bounds = region.bounds.expand(tolerance);

            if expanded_bounds.contains(x, y) {
                // 计算到区域中心点的距离
                let (cx, cy) = region.bounds.center();
                let dx = x - cx;
                let dy = y - cy;
                let distance = (dx * dx + dy * dy).sqrt();

                // 更新最近的命中点
                match &closest {
                    None => {
                        closest = Some((idx, distance, region));
                    }
                    Some((_, prev_dist, _)) => {
                        if distance < *prev_dist {
                            closest = Some((idx, distance, region));
                        }
                    }
                }
            }
        }

        closest.map(|(_, distance, region)| HitResult {
            index: region.index,
            series: region.series,
            distance,
            data: region.data.clone(),
        })
    }

    fn invert(&self, x: f64, y: f64) -> Option<(FieldValue, FieldValue)> {
        // 使用比例尺反向映射到数据空间
        let x_val = self.x_scale.invert(x);
        let y_val = self.y_scale.invert(y);

        Some((FieldValue::Numeric(x_val), FieldValue::Numeric(y_val)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_lookup_creation() {
        let x_scale = LinearScale::new(0.0, 100.0, 0.0, 500.0);
        let y_scale = LinearScale::new(0.0, 50.0, 0.0, 300.0);
        let regions = vec![];

        let lookup = SimpleLookup::new(regions, x_scale, y_scale);

        assert_eq!(lookup.regions().len(), 0);
        assert_eq!(lookup.x_scale().domain(), (0.0, 100.0));
        assert_eq!(lookup.y_scale().domain(), (0.0, 50.0));
    }

    #[test]
    fn test_hit_test_single_region() {
        let x_scale = LinearScale::new(0.0, 100.0, 0.0, 500.0);
        let y_scale = LinearScale::new(0.0, 50.0, 0.0, 300.0);

        let region = HitRegion::from_rect(
            100.0,
            150.0,
            50.0,
            30.0,
            0,
            None,
            vec![FieldValue::Numeric(42.0)],
        );

        let lookup = SimpleLookup::new(vec![region], x_scale, y_scale);

        // 命中中心点
        let result = lookup.hit_test(125.0, 165.0, 0.0).unwrap();
        assert_eq!(result.index, 0);
        assert_eq!(result.series, None);
        assert_eq!(result.data, vec![FieldValue::Numeric(42.0)]);

        // 命中边界
        assert!(lookup.hit_test(100.0, 150.0, 0.0).is_some());
        assert!(lookup.hit_test(150.0, 180.0, 0.0).is_some());

        // 未命中
        assert!(lookup.hit_test(50.0, 50.0, 0.0).is_none());
    }

    #[test]
    fn test_hit_test_with_tolerance() {
        let x_scale = LinearScale::new(0.0, 100.0, 0.0, 500.0);
        let y_scale = LinearScale::new(0.0, 50.0, 0.0, 300.0);

        let region = HitRegion::from_rect(
            100.0,
            150.0,
            50.0,
            30.0,
            0,
            None,
            vec![FieldValue::Numeric(42.0)],
        );

        let lookup = SimpleLookup::new(vec![region], x_scale, y_scale);

        // 不带容差，未命中
        assert!(lookup.hit_test(95.0, 145.0, 0.0).is_none());

        // 带容差，命中
        let result = lookup.hit_test(95.0, 145.0, 10.0).unwrap();
        assert_eq!(result.index, 0);
    }

    #[test]
    fn test_hit_test_multiple_regions() {
        let x_scale = LinearScale::new(0.0, 100.0, 0.0, 500.0);
        let y_scale = LinearScale::new(0.0, 50.0, 0.0, 300.0);

        let region1 = HitRegion::from_rect(
            100.0,
            150.0,
            50.0,
            30.0,
            0,
            None,
            vec![FieldValue::Numeric(1.0)],
        );

        let region2 = HitRegion::from_rect(
            200.0,
            250.0,
            50.0,
            30.0,
            1,
            None,
            vec![FieldValue::Numeric(2.0)],
        );

        let lookup = SimpleLookup::new(vec![region1, region2], x_scale, y_scale);

        // 命中第一个区域
        let result = lookup.hit_test(125.0, 165.0, 0.0).unwrap();
        assert_eq!(result.index, 0);
        assert_eq!(result.data, vec![FieldValue::Numeric(1.0)]);

        // 命中第二个区域
        let result = lookup.hit_test(225.0, 265.0, 0.0).unwrap();
        assert_eq!(result.index, 1);
        assert_eq!(result.data, vec![FieldValue::Numeric(2.0)]);
    }

    #[test]
    fn test_hit_test_closest_region() {
        let x_scale = LinearScale::new(0.0, 100.0, 0.0, 500.0);
        let y_scale = LinearScale::new(0.0, 50.0, 0.0, 300.0);

        // 两个区域有重叠
        let region1 = HitRegion::from_rect(
            100.0,
            100.0,
            100.0,
            100.0,
            0,
            None,
            vec![FieldValue::Numeric(1.0)],
        );

        let region2 = HitRegion::from_rect(
            150.0,
            150.0,
            100.0,
            100.0,
            1,
            None,
            vec![FieldValue::Numeric(2.0)],
        );

        let lookup = SimpleLookup::new(vec![region1, region2], x_scale, y_scale);

        // 重叠区域的点，应该选择距离中心最近的
        let result = lookup.hit_test(175.0, 175.0, 0.0).unwrap();

        // region1 中心 (150, 150)，region2 中心 (200, 200)
        // 点 (175, 175) 距离两个中心都约 35.36，相等距离时会选择先遍历的 region1
        // 所以这里测试的是距离相等时的行为
        assert_eq!(result.index, 0);

        // 测试一个明显更接近 region2 的点
        let result = lookup.hit_test(190.0, 190.0, 0.0).unwrap();
        // (190, 190) 距离 region1 中心约 56.6，距离 region2 中心约 14.1
        // 应该命中 region2
        assert_eq!(result.index, 1);
    }

    #[test]
    fn test_hit_test_with_series() {
        let x_scale = LinearScale::new(0.0, 100.0, 0.0, 500.0);
        let y_scale = LinearScale::new(0.0, 50.0, 0.0, 300.0);

        let region = HitRegion::from_rect(
            100.0,
            150.0,
            50.0,
            30.0,
            5,
            Some(2),
            vec![FieldValue::Numeric(42.0)],
        );

        let lookup = SimpleLookup::new(vec![region], x_scale, y_scale);

        let result = lookup.hit_test(125.0, 165.0, 0.0).unwrap();
        assert_eq!(result.index, 5);
        assert_eq!(result.series, Some(2));
    }

    #[test]
    fn test_invert() {
        let x_scale = LinearScale::new(0.0, 100.0, 0.0, 500.0);
        let y_scale = LinearScale::new(0.0, 50.0, 0.0, 300.0);

        let lookup = SimpleLookup::new(vec![], x_scale, y_scale);

        // 测试 x 轴反转
        // x_scale: 0.0 -> 0.0, 100.0 -> 500.0
        // 所以 250.0 -> 50.0
        let result = lookup.invert(250.0, 150.0).unwrap();
        assert_eq!(result.0, FieldValue::Numeric(50.0));
        assert_eq!(result.1, FieldValue::Numeric(25.0));

        // 测试边界
        let result = lookup.invert(0.0, 0.0).unwrap();
        assert_eq!(result.0, FieldValue::Numeric(0.0));
        assert_eq!(result.1, FieldValue::Numeric(0.0));

        let result = lookup.invert(500.0, 300.0).unwrap();
        assert_eq!(result.0, FieldValue::Numeric(100.0));
        assert_eq!(result.1, FieldValue::Numeric(50.0));
    }

    #[test]
    fn test_distance_calculation() {
        let x_scale = LinearScale::new(0.0, 100.0, 0.0, 500.0);
        let y_scale = LinearScale::new(0.0, 50.0, 0.0, 300.0);

        let region = HitRegion::from_point(100.0, 100.0, 10.0, 0, None, vec![]);

        let lookup = SimpleLookup::new(vec![region], x_scale, y_scale);

        // 命中在中心点，距离为 0
        let result = lookup.hit_test(100.0, 100.0, 0.0).unwrap();
        assert_eq!(result.distance, 0.0);

        // 命中在边界，距离 > 0
        let result = lookup.hit_test(95.0, 100.0, 0.0).unwrap();
        assert!((result.distance - 5.0).abs() < f64::EPSILON);
    }
}
