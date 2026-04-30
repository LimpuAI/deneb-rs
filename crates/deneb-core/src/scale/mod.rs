//! 比例尺系统
//!
//! 提供数据空间到像素空间的映射，支持连续型和离散型比例尺。

/// 数据空间范围 (min, max)
pub type ScaleDomain = (f64, f64);

/// 像素空间范围 (min, max)
pub type ScaleRange = (f64, f64);

/// 比例尺 trait
///
/// 定义比例尺的基本接口。
pub trait Scale: Clone {
    /// 输入类型
    type Input: Clone;

    /// 将输入值映射到像素空间
    fn map(&self, input: Self::Input) -> f64;

    /// 将像素值反向映射到数据空间
    fn invert(&self, output: f64) -> Self::Input;

    /// 获取数据空间的范围 (min, max)
    fn domain(&self) -> ScaleDomain;

    /// 获取像素空间的范围 (min, max)
    fn range(&self) -> ScaleRange;
}

/// 线性比例尺
///
/// 用于连续数值的线性映射。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearScale {
    /// 数据最小值
    min: f64,
    /// 数据最大值
    max: f64,
    /// 像素最小值
    range_min: f64,
    /// 像素最大值
    range_max: f64,
}

impl LinearScale {
    /// 创建新的线性比例尺
    pub fn new(min: f64, max: f64, range_min: f64, range_max: f64) -> Self {
        Self {
            min,
            max,
            range_min,
            range_max,
        }
    }

    /// 从数据范围和像素范围创建
    pub fn from_domain_and_range(domain: (f64, f64), range: (f64, f64)) -> Self {
        Self::new(domain.0, domain.1, range.0, range.1)
    }

    /// 设置数据范围
    pub fn with_domain(mut self, domain: (f64, f64)) -> Self {
        self.min = domain.0;
        self.max = domain.1;
        self
    }

    /// 设置像素范围
    pub fn with_range(mut self, range: (f64, f64)) -> Self {
        self.range_min = range.0;
        self.range_max = range.1;
        self
    }

    /// 获取比例尺的斜率
    pub fn slope(&self) -> f64 {
        if self.max == self.min {
            0.0
        } else {
            (self.range_max - self.range_min) / (self.max - self.min)
        }
    }

    /// 获取比例尺的截距
    pub fn intercept(&self) -> f64 {
        self.range_min - self.slope() * self.min
    }
}

impl Scale for LinearScale {
    type Input = f64;

    fn map(&self, input: f64) -> f64 {
        if self.max == self.min {
            // 常数映射：返回范围中点
            return (self.range_min + self.range_max) / 2.0;
        }

        let t = (input - self.min) / (self.max - self.min);
        self.range_min + t * (self.range_max - self.range_min)
    }

    fn invert(&self, output: f64) -> f64 {
        if self.range_max == self.range_min {
            // 常数映射：返回域中点
            return (self.min + self.max) / 2.0;
        }

        let t = (output - self.range_min) / (self.range_max - self.range_min);
        self.min + t * (self.max - self.min)
    }

    fn domain(&self) -> (f64, f64) {
        (self.min, self.max)
    }

    fn range(&self) -> (f64, f64) {
        (self.range_min, self.range_max)
    }
}

/// 对数比例尺
///
/// 用于连续数值的对数映射。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogScale {
    /// 对数的底数
    base: f64,
    /// 数据最小值
    min: f64,
    /// 数据最大值
    max: f64,
    /// 像素最小值
    range_min: f64,
    /// 像素最大值
    range_max: f64,
}

impl LogScale {
    /// 创建新的对数比例尺
    ///
    /// # Arguments
    ///
    /// * `base` - 对数的底数（必须 > 0 且 ≠ 1）
    /// * `min` - 数据最小值
    /// * `max` - 数据最大值
    /// * `range_min` - 像素最小值
    /// * `range_max` - 像素最大值
    pub fn new(base: f64, min: f64, max: f64, range_min: f64, range_max: f64) -> Self {
        Self {
            base,
            min,
            max,
            range_min,
            range_max,
        }
    }

    /// 从数据范围和像素范围创建（使用底数 10）
    pub fn from_domain_and_range(domain: (f64, f64), range: (f64, f64)) -> Self {
        Self::new(10.0, domain.0, domain.1, range.0, range.1)
    }

    /// 设置对数底数
    pub fn with_base(mut self, base: f64) -> Self {
        self.base = base;
        self
    }

    /// 设置数据范围
    pub fn with_domain(mut self, domain: (f64, f64)) -> Self {
        self.min = domain.0;
        self.max = domain.1;
        self
    }

    /// 设置像素范围
    pub fn with_range(mut self, range: (f64, f64)) -> Self {
        self.range_min = range.0;
        self.range_max = range.1;
        self
    }

    /// 将值 clamp 到正数范围
    fn clamp_positive(&self, value: f64) -> f64 {
        if value <= 0.0 {
            // 返回一个合理的最小正数，避免对数产生负无穷
            self.min.max(1e-10)
        } else {
            value
        }
    }

    /// 计算对数
    fn log(&self, value: f64) -> f64 {
        value.log(self.base)
    }

    /// 计算指数
    fn exp(&self, value: f64) -> f64 {
        self.base.powf(value)
    }
}

impl Scale for LogScale {
    type Input = f64;

    fn map(&self, input: f64) -> f64 {
        let clamped_input = self.clamp_positive(input);

        if self.max == self.min {
            return (self.range_min + self.range_max) / 2.0;
        }

        let log_min = self.log(self.clamp_positive(self.min));
        let log_max = self.log(self.clamp_positive(self.max));
        let log_input = self.log(clamped_input);

        if log_max == log_min {
            return (self.range_min + self.range_max) / 2.0;
        }

        let t = (log_input - log_min) / (log_max - log_min);
        self.range_min + t * (self.range_max - self.range_min)
    }

    fn invert(&self, output: f64) -> f64 {
        if self.range_max == self.range_min {
            return (self.min + self.max) / 2.0;
        }

        let t = (output - self.range_min) / (self.range_max - self.range_min);

        let log_min = self.log(self.clamp_positive(self.min));
        let log_max = self.log(self.clamp_positive(self.max));

        let log_value = log_min + t * (log_max - log_min);
        self.exp(log_value)
    }

    fn domain(&self) -> (f64, f64) {
        (self.min, self.max)
    }

    fn range(&self) -> (f64, f64) {
        (self.range_min, self.range_max)
    }
}

/// 时间比例尺
///
/// 本质是线性比例尺，但专门用于时间戳。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeScale {
    /// 内部线性比例尺
    inner: LinearScale,
}

impl TimeScale {
    /// 创建新的时间比例尺
    pub fn new(min: f64, max: f64, range_min: f64, range_max: f64) -> Self {
        Self {
            inner: LinearScale::new(min, max, range_min, range_max),
        }
    }

    /// 从数据范围和像素范围创建
    pub fn from_domain_and_range(domain: (f64, f64), range: (f64, f64)) -> Self {
        Self {
            inner: LinearScale::from_domain_and_range(domain, range),
        }
    }

    /// 设置数据范围
    pub fn with_domain(mut self, domain: (f64, f64)) -> Self {
        self.inner = self.inner.with_domain(domain);
        self
    }

    /// 设置像素范围
    pub fn with_range(mut self, range: (f64, f64)) -> Self {
        self.inner = self.inner.with_range(range);
        self
    }
}

impl Scale for TimeScale {
    type Input = f64;

    fn map(&self, input: f64) -> f64 {
        self.inner.map(input)
    }

    fn invert(&self, output: f64) -> f64 {
        self.inner.invert(output)
    }

    fn domain(&self) -> (f64, f64) {
        self.inner.domain()
    }

    fn range(&self) -> (f64, f64) {
        self.inner.range()
    }
}

/// 序数比例尺
///
/// 用于离散类别的映射。
#[derive(Debug, Clone, PartialEq)]
pub struct OrdinalScale {
    /// 类别值
    values: Vec<String>,
    /// 像素范围
    range: Vec<f64>,
}

impl OrdinalScale {
    /// 创建新的序数比例尺
    ///
    /// # Arguments
    ///
    /// * `values` - 类别值列表
    /// * `range` - 像素范围 [min, max]
    pub fn new(values: Vec<String>, range: (f64, f64)) -> Self {
        let n = values.len();
        let step = if n > 1 {
            (range.1 - range.0) / (n - 1) as f64
        } else {
            0.0
        };

        let range_values: Vec<f64> = (0..n)
            .map(|i| range.0 + i as f64 * step)
            .collect();

        Self {
            values,
            range: range_values,
        }
    }

    /// 设置类别值
    pub fn with_values(self, values: Vec<String>) -> Self {
        let range = if self.range.len() >= 2 {
            (self.range[0], self.range.last().copied().unwrap_or(0.0))
        } else {
            (0.0, 1.0)
        };
        Self::new(values, range)
    }

    /// 设置像素范围
    pub fn with_range(self, range: (f64, f64)) -> Self {
        Self::new(self.values.clone(), range)
    }

    /// 获取类别的索引
    fn index_of(&self, value: &str) -> Option<usize> {
        self.values.iter().position(|v| v == value)
    }
}

impl Scale for OrdinalScale {
    type Input = String;

    fn map(&self, input: String) -> f64 {
        if let Some(index) = self.index_of(&input) {
            if let Some(&pos) = self.range.get(index) {
                return pos;
            }
        }
        // 如果找不到，返回范围中点
        if self.range.is_empty() {
            0.0
        } else if self.range.len() == 1 {
            self.range[0]
        } else {
            (self.range[0] + self.range.last().unwrap()) / 2.0
        }
    }

    fn invert(&self, output: f64) -> String {
        // 找到最接近的类别
        if self.range.is_empty() {
            return String::new();
        }

        let mut closest_index = 0;
        let mut closest_dist = (output - self.range[0]).abs();

        for (i, &pos) in self.range.iter().enumerate() {
            let dist = (output - pos).abs();
            if dist < closest_dist {
                closest_dist = dist;
                closest_index = i;
            }
        }

        self.values
            .get(closest_index)
            .cloned()
            .unwrap_or_default()
    }

    fn domain(&self) -> (f64, f64) {
        (0.0, self.values.len() as f64)
    }

    fn range(&self) -> (f64, f64) {
        if self.range.is_empty() {
            (0.0, 1.0)
        } else {
            (self.range[0], *self.range.last().unwrap_or(&1.0))
        }
    }
}

/// 条形比例尺
///
/// 用于条形图的类别轴，提供带状区域。
#[derive(Debug, Clone, PartialEq)]
pub struct BandScale {
    /// 类别值
    values: Vec<String>,
    /// 像素最小值
    range_min: f64,
    /// 像素最大值
    range_max: f64,
    /// 内部边距（比例，0-1）
    padding: f64,
}

impl BandScale {
    /// 创建新的条形比例尺
    ///
    /// # Arguments
    ///
    /// * `values` - 类别值列表
    /// * `range_min` - 像素最小值
    /// * `range_max` - 像素最大值
    /// * `padding` - 内部边距（0-1），默认 0.1
    pub fn new(values: Vec<String>, range_min: f64, range_max: f64, padding: f64) -> Self {
        Self {
            values,
            range_min,
            range_max,
            padding: padding.clamp(0.0, 1.0),
        }
    }

    /// 从数据范围和像素范围创建
    pub fn from_domain_and_range(
        values: Vec<String>,
        range: (f64, f64),
    ) -> Self {
        Self::new(values, range.0, range.1, 0.1)
    }

    /// 设置类别值
    pub fn with_values(mut self, values: Vec<String>) -> Self {
        self.values = values;
        self
    }

    /// 设置像素范围
    pub fn with_range(mut self, range: (f64, f64)) -> Self {
        self.range_min = range.0;
        self.range_max = range.1;
        self
    }

    /// 设置边距
    pub fn with_padding(mut self, padding: f64) -> Self {
        self.padding = padding.clamp(0.0, 1.0);
        self
    }

    /// 获取条带宽度
    pub fn band_width(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }

        let total_range = self.range_max - self.range_min;
        let n = self.values.len() as f64;
        let step = total_range / n;

        step * (1.0 - self.padding)
    }

    /// 获取条带步长（包括边距）
    pub fn step_width(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }

        (self.range_max - self.range_min) / self.values.len() as f64
    }

    /// 获取某个类别的起始 x 坐标
    pub fn band_start(&self, value: &str) -> Option<f64> {
        let index = self.values.iter().position(|v| v == value)?;
        let step = self.step_width();
        let band_width = self.band_width();
        let padding = (step - band_width) / 2.0;

        Some(self.range_min + index as f64 * step + padding)
    }

    /// 获取某个类别的中心 x 坐标
    pub fn band_center(&self, value: &str) -> Option<f64> {
        let start = self.band_start(value)?;
        Some(start + self.band_width() / 2.0)
    }
}

impl Scale for BandScale {
    type Input = String;

    fn map(&self, input: String) -> f64 {
        self.band_center(&input)
            .unwrap_or_else(|| (self.range_min + self.range_max) / 2.0)
    }

    fn invert(&self, output: f64) -> String {
        if self.values.is_empty() {
            return String::new();
        }

        let step = self.step_width();
        let relative_pos = output - self.range_min;
        let index = (relative_pos / step).floor() as usize;

        self.values
            .get(index.min(self.values.len().saturating_sub(1)))
            .cloned()
            .unwrap_or_default()
    }

    fn domain(&self) -> (f64, f64) {
        (0.0, self.values.len() as f64)
    }

    fn range(&self) -> (f64, f64) {
        (self.range_min, self.range_max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_scale() {
        let scale = LinearScale::new(0.0, 100.0, 0.0, 500.0);

        assert_eq!(scale.map(0.0), 0.0);
        assert_eq!(scale.map(50.0), 250.0);
        assert_eq!(scale.map(100.0), 500.0);

        assert_eq!(scale.invert(0.0), 0.0);
        assert_eq!(scale.invert(250.0), 50.0);
        assert_eq!(scale.invert(500.0), 100.0);

        assert_eq!(scale.domain(), (0.0, 100.0));
        assert_eq!(scale.range(), (0.0, 500.0));
    }

    #[test]
    fn test_linear_scale_constant() {
        let scale = LinearScale::new(50.0, 50.0, 0.0, 500.0);

        // 常数映射应该返回范围中点
        assert_eq!(scale.map(0.0), 250.0);
        assert_eq!(scale.map(100.0), 250.0);
        assert_eq!(scale.invert(250.0), 50.0);
    }

    #[test]
    fn test_linear_scale_slope_intercept() {
        let scale = LinearScale::new(0.0, 100.0, 0.0, 500.0);

        assert!((scale.slope() - 5.0).abs() < f64::EPSILON);
        assert!((scale.intercept() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_log_scale() {
        let scale = LogScale::new(10.0, 1.0, 1000.0, 0.0, 500.0);

        // 对数比例尺：log10(1) = 0, log10(10) = 1, log10(100) = 2, log10(1000) = 3
        // 所以 1 -> 0.0, 10 -> 166.67 (1/3), 100 -> 333.33 (2/3), 1000 -> 500.0 (3/3)
        assert!((scale.map(1.0) - 0.0).abs() < 1e-9);
        assert!((scale.map(10.0) - 500.0 / 3.0).abs() < 1e-9);
        assert!((scale.map(100.0) - 1000.0 / 3.0).abs() < 1e-9);
        assert!((scale.map(1000.0) - 500.0).abs() < 1e-9);

        assert!((scale.invert(0.0) - 1.0).abs() < 1e-9);
        assert!((scale.invert(500.0 / 3.0) - 10.0).abs() < 1e-8);
        assert!((scale.invert(1000.0 / 3.0) - 100.0).abs() < 1e-8);
        assert!((scale.invert(500.0) - 1000.0).abs() < 1e-8);
    }

    #[test]
    fn test_log_scale_clamp_negative() {
        let scale = LogScale::new(10.0, 1.0, 100.0, 0.0, 500.0);

        // 负值和零应该被 clamp 到正数
        let result = scale.map(-10.0);
        assert!(!result.is_nan());
        // clamp 后的值会产生有效的对数映射
        assert!(result >= 0.0);

        let result = scale.map(0.0);
        assert!(!result.is_nan());
        // clamp 后的值会产生有效的对数映射
        assert!(result >= 0.0);
    }

    #[test]
    fn test_time_scale() {
        let scale = TimeScale::new(0.0, 86400.0, 0.0, 1000.0);

        assert_eq!(scale.map(0.0), 0.0);
        assert_eq!(scale.map(43200.0), 500.0); // 中午 12 点
        assert_eq!(scale.map(86400.0), 1000.0);

        assert_eq!(scale.invert(0.0), 0.0);
        assert_eq!(scale.invert(500.0), 43200.0);
        assert_eq!(scale.invert(1000.0), 86400.0);
    }

    #[test]
    fn test_ordinal_scale() {
        let scale = OrdinalScale::new(
            vec!["A".to_string(), "B".to_string(), "C".to_string()],
            (0.0, 200.0),
        );

        assert_eq!(scale.map("A".to_string()), 0.0);
        assert_eq!(scale.map("B".to_string()), 100.0);
        assert_eq!(scale.map("C".to_string()), 200.0);

        assert_eq!(scale.invert(0.0), "A");
        assert_eq!(scale.invert(100.0), "B");
        assert_eq!(scale.invert(200.0), "C");

        // 找不到的值应该返回范围中点
        assert_eq!(scale.map("X".to_string()), 100.0);
    }

    #[test]
    fn test_band_scale() {
        let scale = BandScale::new(
            vec!["A".to_string(), "B".to_string(), "C".to_string()],
            0.0,
            300.0,
            0.1,
        );

        assert_eq!(scale.step_width(), 100.0);
        assert!((scale.band_width() - 90.0).abs() < f64::EPSILON);

        // 每个条带宽度 90，步长 100，padding 各 5
        assert_eq!(scale.band_start("A"), Some(5.0));
        assert_eq!(scale.band_start("B"), Some(105.0));
        assert_eq!(scale.band_start("C"), Some(205.0));

        // 中心点
        assert_eq!(scale.band_center("A"), Some(50.0));
        assert_eq!(scale.band_center("B"), Some(150.0));
        assert_eq!(scale.band_center("C"), Some(250.0));

        // map 应该返回中心点
        assert_eq!(scale.map("A".to_string()), 50.0);
        assert_eq!(scale.map("B".to_string()), 150.0);
        assert_eq!(scale.map("C".to_string()), 250.0);
    }

    #[test]
    fn test_band_scale_invert() {
        let scale = BandScale::new(
            vec!["A".to_string(), "B".to_string(), "C".to_string()],
            0.0,
            300.0,
            0.1,
        );

        // 中心点应该 invert 到对应的类别
        assert_eq!(scale.invert(50.0), "A");
        assert_eq!(scale.invert(150.0), "B");
        assert_eq!(scale.invert(250.0), "C");
    }

    #[test]
    fn test_band_scale_empty() {
        let scale = BandScale::new(vec![], 0.0, 300.0, 0.1);

        assert_eq!(scale.band_width(), 0.0);
        assert_eq!(scale.step_width(), 0.0);
        assert_eq!(scale.band_start("A"), None);
        assert_eq!(scale.map("A".to_string()), 150.0);
    }

    #[test]
    fn test_scale_builder_methods() {
        let scale = LinearScale::new(0.0, 100.0, 0.0, 500.0)
            .with_domain((10.0, 90.0))
            .with_range((100.0, 400.0));

        assert_eq!(scale.domain(), (10.0, 90.0));
        assert_eq!(scale.range(), (100.0, 400.0));
    }

    #[test]
    fn test_log_scale_with_different_base() {
        let scale2 = LogScale::new(2.0, 1.0, 8.0, 0.0, 300.0);

        assert!((scale2.map(1.0) - 0.0).abs() < f64::EPSILON);
        assert!((scale2.map(2.0) - 100.0).abs() < f64::EPSILON);
        assert!((scale2.map(4.0) - 200.0).abs() < f64::EPSILON);
        assert!((scale2.map(8.0) - 300.0).abs() < f64::EPSILON);
    }
}
