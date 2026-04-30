//! ChartSpec builder API
//!
//! 提供声明式图表规格定义，启发自 Vega-Lite。

use deneb_core::DataType;
use std::fmt;

/// Mark 类型
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mark {
    /// 折线
    Line,
    /// 柱状
    Bar,
    /// 散点
    Scatter,
    /// 面积
    Area,
}

impl fmt::Display for Mark {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mark::Line => write!(f, "line"),
            Mark::Bar => write!(f, "bar"),
            Mark::Scatter => write!(f, "scatter"),
            Mark::Area => write!(f, "area"),
        }
    }
}

/// 聚合函数
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Aggregate {
    /// 求和
    Sum,
    /// 平均值
    Mean,
    /// 中位数
    Median,
    /// 最小值
    Min,
    /// 最大值
    Max,
    /// 计数
    Count,
}

impl fmt::Display for Aggregate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Aggregate::Sum => write!(f, "sum"),
            Aggregate::Mean => write!(f, "mean"),
            Aggregate::Median => write!(f, "median"),
            Aggregate::Min => write!(f, "min"),
            Aggregate::Max => write!(f, "max"),
            Aggregate::Count => write!(f, "count"),
        }
    }
}

/// 字段编码定义
#[derive(Clone, Debug)]
pub struct Field {
    /// 字段名
    pub name: String,
    /// 数据类型
    pub data_type: DataType,
    /// 聚合函数
    pub aggregate: Option<Aggregate>,
    /// 显示标题
    pub title: Option<String>,
}

impl Field {
    /// 创建定量字段
    pub fn quantitative(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_type: DataType::Quantitative,
            aggregate: None,
            title: None,
        }
    }

    /// 创建时间字段
    pub fn temporal(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_type: DataType::Temporal,
            aggregate: None,
            title: None,
        }
    }

    /// 创建名义字段
    pub fn nominal(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_type: DataType::Nominal,
            aggregate: None,
            title: None,
        }
    }

    /// 创建序数字段
    pub fn ordinal(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_type: DataType::Ordinal,
            aggregate: None,
            title: None,
        }
    }

    /// 设置聚合函数
    pub fn with_aggregate(mut self, agg: Aggregate) -> Self {
        self.aggregate = Some(agg);
        self
    }

    /// 设置标题
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// 编码通道
#[derive(Clone, Debug, Default)]
pub struct Encoding {
    /// X 通道
    pub x: Option<Field>,
    /// Y 通道
    pub y: Option<Field>,
    /// 颜色通道
    pub color: Option<Field>,
    /// 大小通道
    pub size: Option<Field>,
}

impl Encoding {
    /// 创建新的编码
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置 X 通道
    pub fn x(mut self, field: Field) -> Self {
        self.x = Some(field);
        self
    }

    /// 设置 Y 通道
    pub fn y(mut self, field: Field) -> Self {
        self.y = Some(field);
        self
    }

    /// 设置颜色通道
    pub fn color(mut self, field: Field) -> Self {
        self.color = Some(field);
        self
    }

    /// 设置大小通道
    pub fn size(mut self, field: Field) -> Self {
        self.size = Some(field);
        self
    }
}

/// 图表规格
#[derive(Clone, Debug)]
pub struct ChartSpec {
    /// Mark 类型
    pub mark: Mark,
    /// 编码配置
    pub encoding: Encoding,
    /// 标题
    pub title: Option<String>,
    /// 宽度
    pub width: f64,
    /// 高度
    pub height: f64,
}

/// ChartSpec Builder
pub struct ChartSpecBuilder {
    mark: Option<Mark>,
    encoding: Encoding,
    title: Option<String>,
    width: f64,
    height: f64,
}

impl ChartSpecBuilder {
    /// 创建新的 builder
    pub fn new() -> Self {
        Self {
            mark: None,
            encoding: Encoding::new(),
            title: None,
            width: 400.0,
            height: 300.0,
        }
    }

    /// 设置 mark 类型
    pub fn mark(mut self, mark: Mark) -> Self {
        self.mark = Some(mark);
        self
    }

    /// 设置编码
    pub fn encoding(mut self, encoding: Encoding) -> Self {
        self.encoding = encoding;
        self
    }

    /// 设置标题
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// 设置宽度
    pub fn width(mut self, width: f64) -> Self {
        self.width = width;
        self
    }

    /// 设置高度
    pub fn height(mut self, height: f64) -> Self {
        self.height = height;
        self
    }

    /// 构建 ChartSpec
    pub fn build(self) -> Result<ChartSpec, crate::error::ComponentError> {
        // 验证 mark 必须设置
        let mark = self.mark.ok_or_else(|| {
            crate::error::ComponentError::InvalidConfig {
                reason: "mark is required".to_string(),
            }
        })?;

        // 验证 encoding.x 和 encoding.y 必须设置
        if self.encoding.x.is_none() {
            return Err(crate::error::ComponentError::InvalidConfig {
                reason: "encoding.x is required".to_string(),
            });
        }

        if self.encoding.y.is_none() {
            return Err(crate::error::ComponentError::InvalidConfig {
                reason: "encoding.y is required".to_string(),
            });
        }

        // 验证 width > 0
        if self.width <= 0.0 {
            return Err(crate::error::ComponentError::InvalidConfig {
                reason: format!("width must be positive, got {}", self.width),
            });
        }

        // 验证 height > 0
        if self.height <= 0.0 {
            return Err(crate::error::ComponentError::InvalidConfig {
                reason: format!("height must be positive, got {}", self.height),
            });
        }

        Ok(ChartSpec {
            mark,
            encoding: self.encoding,
            title: self.title,
            width: self.width,
            height: self.height,
        })
    }
}

impl Default for ChartSpecBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ChartSpec {
    /// 创建 builder
    pub fn builder() -> ChartSpecBuilder {
        ChartSpecBuilder::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mark_display() {
        assert_eq!(Mark::Line.to_string(), "line");
        assert_eq!(Mark::Bar.to_string(), "bar");
        assert_eq!(Mark::Scatter.to_string(), "scatter");
        assert_eq!(Mark::Area.to_string(), "area");
    }

    #[test]
    fn test_aggregate_display() {
        assert_eq!(Aggregate::Sum.to_string(), "sum");
        assert_eq!(Aggregate::Mean.to_string(), "mean");
        assert_eq!(Aggregate::Median.to_string(), "median");
        assert_eq!(Aggregate::Min.to_string(), "min");
        assert_eq!(Aggregate::Max.to_string(), "max");
        assert_eq!(Aggregate::Count.to_string(), "count");
    }

    #[test]
    fn test_field_constructors() {
        let q_field = Field::quantitative("value");
        assert_eq!(q_field.name, "value");
        assert_eq!(q_field.data_type, DataType::Quantitative);
        assert!(q_field.aggregate.is_none());
        assert!(q_field.title.is_none());

        let t_field = Field::temporal("date");
        assert_eq!(t_field.data_type, DataType::Temporal);

        let n_field = Field::nominal("category");
        assert_eq!(n_field.data_type, DataType::Nominal);

        let o_field = Field::ordinal("rating");
        assert_eq!(o_field.data_type, DataType::Ordinal);
    }

    #[test]
    fn test_field_builders() {
        let field = Field::quantitative("price")
            .with_aggregate(Aggregate::Mean)
            .with_title("Average Price");

        assert_eq!(field.name, "price");
        assert_eq!(field.data_type, DataType::Quantitative);
        assert_eq!(field.aggregate, Some(Aggregate::Mean));
        assert_eq!(field.title, Some("Average Price".to_string()));
    }

    #[test]
    fn test_encoding_builders() {
        let encoding = Encoding::new()
            .x(Field::temporal("date"))
            .y(Field::quantitative("value"))
            .color(Field::nominal("category"))
            .size(Field::quantitative("size"));

        assert!(encoding.x.is_some());
        assert!(encoding.y.is_some());
        assert!(encoding.color.is_some());
        assert!(encoding.size.is_some());
        assert_eq!(encoding.x.as_ref().unwrap().name, "date");
    }

    #[test]
    fn test_chart_spec_builder_success() {
        let spec = ChartSpec::builder()
            .mark(Mark::Line)
            .encoding(
                Encoding::new()
                    .x(Field::temporal("date"))
                    .y(Field::quantitative("value")),
            )
            .title("Sales Trend")
            .width(800.0)
            .height(400.0)
            .build()
            .unwrap();

        assert_eq!(spec.mark, Mark::Line);
        assert_eq!(spec.title, Some("Sales Trend".to_string()));
        assert_eq!(spec.width, 800.0);
        assert_eq!(spec.height, 400.0);
        assert_eq!(spec.encoding.x.as_ref().unwrap().name, "date");
        assert_eq!(spec.encoding.y.as_ref().unwrap().name, "value");
    }

    #[test]
    fn test_chart_spec_builder_missing_mark() {
        let result = ChartSpec::builder()
            .encoding(
                Encoding::new()
                    .x(Field::temporal("date"))
                    .y(Field::quantitative("value")),
            )
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(
            err.to_string(),
            "Invalid configuration: mark is required"
        );
    }

    #[test]
    fn test_chart_spec_builder_missing_x() {
        let result = ChartSpec::builder()
            .mark(Mark::Line)
            .encoding(Encoding::new().y(Field::quantitative("value")))
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(
            err.to_string(),
            "Invalid configuration: encoding.x is required"
        );
    }

    #[test]
    fn test_chart_spec_builder_missing_y() {
        let result = ChartSpec::builder()
            .mark(Mark::Line)
            .encoding(Encoding::new().x(Field::temporal("date")))
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(
            err.to_string(),
            "Invalid configuration: encoding.y is required"
        );
    }

    #[test]
    fn test_chart_spec_builder_invalid_width() {
        let result = ChartSpec::builder()
            .mark(Mark::Line)
            .encoding(
                Encoding::new()
                    .x(Field::temporal("date"))
                    .y(Field::quantitative("value")),
            )
            .width(-100.0)
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("width must be positive"));
    }

    #[test]
    fn test_chart_spec_builder_invalid_height() {
        let result = ChartSpec::builder()
            .mark(Mark::Line)
            .encoding(
                Encoding::new()
                    .x(Field::temporal("date"))
                    .y(Field::quantitative("value")),
            )
            .height(0.0)
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("height must be positive"));
    }

    #[test]
    fn test_chart_spec_builder_default_size() {
        let spec = ChartSpec::builder()
            .mark(Mark::Bar)
            .encoding(
                Encoding::new()
                    .x(Field::nominal("category"))
                    .y(Field::quantitative("count")),
            )
            .build()
            .unwrap();

        // 默认尺寸
        assert_eq!(spec.width, 400.0);
        assert_eq!(spec.height, 300.0);
    }

    #[test]
    fn test_chart_spec_clone() {
        let spec1 = ChartSpec::builder()
            .mark(Mark::Line)
            .encoding(
                Encoding::new()
                    .x(Field::temporal("date"))
                    .y(Field::quantitative("value")),
            )
            .build()
            .unwrap();

        let spec2 = spec1.clone();
        assert_eq!(spec1.mark, spec2.mark);
        assert_eq!(spec1.width, spec2.width);
    }
}
