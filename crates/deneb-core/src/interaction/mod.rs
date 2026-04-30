//! 交互支持模块
//!
//! 提供命中检测和坐标反查能力，支持图表的交互功能。

mod lookup;

pub use lookup::{CoordLookup, SimpleLookup};

use crate::data::FieldValue;

/// 轴对齐包围盒
///
/// 表示矩形区域的几何边界，用于命中检测。
#[derive(Clone, Debug, PartialEq)]
pub struct BoundingBox {
    /// 左上角 x 坐标
    pub x: f64,
    /// 左上角 y 坐标
    pub y: f64,
    /// 宽度
    pub width: f64,
    /// 高度
    pub height: f64,
}

impl BoundingBox {
    /// 创建新的包围盒
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width: width.max(0.0),
            height: height.max(0.0),
        }
    }

    /// 判断点是否在包围盒内
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

    /// 获取包围盒中心点
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// 扩大包围盒（四周均匀扩展）
    pub fn expand(&self, padding: f64) -> BoundingBox {
        BoundingBox::new(
            self.x - padding,
            self.y - padding,
            self.width + 2.0 * padding,
            self.height + 2.0 * padding,
        )
    }

    /// 判断与另一个包围盒是否相交
    pub fn intersects(&self, other: &BoundingBox) -> bool {
        !(self.x > other.x + other.width
            || self.x + self.width < other.x
            || self.y > other.y + other.height
            || self.y + self.height < other.y)
    }

    /// 计算包围盒面积
    pub fn area(&self) -> f64 {
        self.width * self.height
    }
}

/// 命中区域 — 每个数据点对应的可交互区域
///
/// 存储数据点的几何位置和对应的数据值，用于交互检测。
#[derive(Clone, Debug)]
pub struct HitRegion {
    /// 数据点索引
    pub index: usize,
    /// 系列索引（多系列时）
    pub series: Option<usize>,
    /// 包围盒
    pub bounds: BoundingBox,
    /// 对应的数据值（该行各列的值）
    pub data: Vec<FieldValue>,
}

impl HitRegion {
    /// 创建新的命中区域
    pub fn new(
        index: usize,
        series: Option<usize>,
        bounds: BoundingBox,
        data: Vec<FieldValue>,
    ) -> Self {
        Self {
            index,
            series,
            bounds,
            data,
        }
    }

    /// 判断点是否在命中区域内
    pub fn hit_test(&self, x: f64, y: f64) -> bool {
        self.bounds.contains(x, y)
    }

    /// 从点创建圆形包围盒的命中区域
    ///
    /// # Arguments
    ///
    /// * `cx` - 中心点 x 坐标
    /// * `cy` - 中心点 y 坐标
    /// * `radius` - 半径
    /// * `index` - 数据点索引
    /// * `series` - 系列索引
    /// * `data` - 数据值
    pub fn from_point(
        cx: f64,
        cy: f64,
        radius: f64,
        index: usize,
        series: Option<usize>,
        data: Vec<FieldValue>,
    ) -> Self {
        let bounds = BoundingBox::new(cx - radius, cy - radius, 2.0 * radius, 2.0 * radius);
        Self::new(index, series, bounds, data)
    }

    /// 从矩形创建命中区域
    ///
    /// # Arguments
    ///
    /// * `x` - 左上角 x 坐标
    /// * `y` - 左上角 y 坐标
    /// * `width` - 宽度
    /// * `height` - 高度
    /// * `index` - 数据点索引
    /// * `series` - 系列索引
    /// * `data` - 数据值
    pub fn from_rect(
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        index: usize,
        series: Option<usize>,
        data: Vec<FieldValue>,
    ) -> Self {
        let bounds = BoundingBox::new(x, y, width, height);
        Self::new(index, series, bounds, data)
    }
}

/// 命中测试结果
///
/// 表示命中检测的返回结果，包含命中点的详细信息。
#[derive(Clone, Debug)]
pub struct HitResult {
    /// 数据点索引
    pub index: usize,
    /// 系列索引
    pub series: Option<usize>,
    /// 到命中点的距离
    pub distance: f64,
    /// 命中区域引用（用值，因为需要 Clone）
    pub data: Vec<FieldValue>,
}

impl HitResult {
    /// 创建新的命中结果
    pub fn new(index: usize, series: Option<usize>, distance: f64, data: Vec<FieldValue>) -> Self {
        Self {
            index,
            series,
            distance,
            data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounding_box_creation() {
        let bbox = BoundingBox::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(bbox.x, 10.0);
        assert_eq!(bbox.y, 20.0);
        assert_eq!(bbox.width, 100.0);
        assert_eq!(bbox.height, 50.0);
    }

    #[test]
    fn test_bounding_box_contains() {
        let bbox = BoundingBox::new(10.0, 20.0, 100.0, 50.0);

        // 内部点
        assert!(bbox.contains(50.0, 40.0));
        assert!(bbox.contains(10.0, 20.0)); // 左上角
        assert!(bbox.contains(110.0, 70.0)); // 右下角

        // 外部点
        assert!(!bbox.contains(5.0, 40.0));
        assert!(!bbox.contains(50.0, 15.0));
        assert!(!bbox.contains(120.0, 40.0));
        assert!(!bbox.contains(50.0, 80.0));
    }

    #[test]
    fn test_bounding_box_center() {
        let bbox = BoundingBox::new(10.0, 20.0, 100.0, 50.0);
        let center = bbox.center();
        assert_eq!(center, (60.0, 45.0));
    }

    #[test]
    fn test_bounding_box_expand() {
        let bbox = BoundingBox::new(10.0, 20.0, 100.0, 50.0);
        let expanded = bbox.expand(5.0);

        assert_eq!(expanded.x, 5.0);
        assert_eq!(expanded.y, 15.0);
        assert_eq!(expanded.width, 110.0);
        assert_eq!(expanded.height, 60.0);
    }

    #[test]
    fn test_bounding_box_intersects() {
        let bbox1 = BoundingBox::new(10.0, 20.0, 100.0, 50.0);
        let bbox2 = BoundingBox::new(50.0, 30.0, 100.0, 50.0);
        let bbox3 = BoundingBox::new(120.0, 80.0, 50.0, 30.0);

        assert!(bbox1.intersects(&bbox2));
        assert!(!bbox1.intersects(&bbox3));
    }

    #[test]
    fn test_bounding_box_area() {
        let bbox = BoundingBox::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(bbox.area(), 5000.0);
    }

    #[test]
    fn test_bounding_box_negative_dimensions() {
        let bbox = BoundingBox::new(10.0, 20.0, -10.0, -5.0);
        // 负值维度应该被 clamp 到 0
        assert_eq!(bbox.width, 0.0);
        assert_eq!(bbox.height, 0.0);
    }

    #[test]
    fn test_hit_region_creation() {
        let bounds = BoundingBox::new(10.0, 20.0, 100.0, 50.0);
        let data = vec![FieldValue::Numeric(42.0)];

        let region = HitRegion::new(5, Some(2), bounds.clone(), data.clone());

        assert_eq!(region.index, 5);
        assert_eq!(region.series, Some(2));
        assert_eq!(region.bounds, bounds);
        assert_eq!(region.data, data);
    }

    #[test]
    fn test_hit_region_hit_test() {
        let bounds = BoundingBox::new(10.0, 20.0, 100.0, 50.0);
        let region = HitRegion::new(0, None, bounds, vec![]);

        assert!(region.hit_test(50.0, 40.0));
        assert!(!region.hit_test(5.0, 40.0));
    }

    #[test]
    fn test_hit_region_from_point() {
        let data = vec![FieldValue::Numeric(42.0)];
        let region = HitRegion::from_point(50.0, 40.0, 10.0, 0, None, data.clone());

        assert_eq!(region.bounds, BoundingBox::new(40.0, 30.0, 20.0, 20.0));
        assert!(region.hit_test(50.0, 40.0)); // 中心点
        assert!(region.hit_test(40.0, 30.0)); // 左上角
        assert!(!region.hit_test(30.0, 20.0)); // 外部
    }

    #[test]
    fn test_hit_region_from_rect() {
        let data = vec![FieldValue::Numeric(42.0)];
        let region = HitRegion::from_rect(10.0, 20.0, 100.0, 50.0, 3, Some(1), data.clone());

        assert_eq!(region.bounds, BoundingBox::new(10.0, 20.0, 100.0, 50.0));
        assert_eq!(region.index, 3);
        assert_eq!(region.series, Some(1));
    }

    #[test]
    fn test_hit_result_creation() {
        let data = vec![FieldValue::Numeric(42.0), FieldValue::Text("test".to_string())];
        let result = HitResult::new(5, Some(2), 3.5, data.clone());

        assert_eq!(result.index, 5);
        assert_eq!(result.series, Some(2));
        assert_eq!(result.distance, 3.5);
        assert_eq!(result.data, data);
    }
}
