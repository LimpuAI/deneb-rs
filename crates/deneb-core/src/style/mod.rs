//! Style 类型定义
//!
//! 提供可视化中使用的各种样式类型，包括填充、描边、渐变、文本样式等。

use std::fmt;

/// 填充样式
#[derive(Clone, Debug, PartialEq)]
pub enum FillStyle {
    /// 纯色填充，CSS 颜色字符串: "#fff", "rgb(255,255,255)", "rgba(...)"
    Color(String),
    /// 渐变填充
    Gradient(Gradient),
    /// 无填充
    None,
}

/// 描边样式
#[derive(Clone, Debug, PartialEq)]
pub enum StrokeStyle {
    /// 纯色描边，CSS 颜色字符串
    Color(String),
    /// 无描边
    None,
}

/// 渐变定义
#[derive(Clone, Debug, PartialEq)]
pub struct Gradient {
    /// 渐变类型
    pub kind: GradientKind,
    /// 渐变停止点
    pub stops: Vec<GradientStop>,
}

/// 渐变类型
#[derive(Clone, Debug, PartialEq)]
pub enum GradientKind {
    /// 线性渐变
    Linear {
        /// 起点 x 坐标
        x0: f64,
        /// 起点 y 坐标
        y0: f64,
        /// 终点 x 坐标
        x1: f64,
        /// 终点 y 坐标
        y1: f64,
    },
    /// 径向渐变
    Radial {
        /// 内圆心 x 坐标
        x0: f64,
        /// 内圆心 y 坐标
        y0: f64,
        /// 内圆半径
        r0: f64,
        /// 外圆心 x 坐标
        x1: f64,
        /// 外圆心 y 坐标
        y1: f64,
        /// 外圆半径
        r1: f64,
    },
}

/// 渐变停止点
#[derive(Clone, Debug, PartialEq)]
pub struct GradientStop {
    /// 偏移量 (0.0 - 1.0)
    pub offset: f64,
    /// 颜色字符串
    pub color: String,
}

impl GradientStop {
    /// 创建新的渐变停止点
    pub fn new(offset: f64, color: impl Into<String>) -> Self {
        Self {
            offset: offset.clamp(0.0, 1.0),
            color: color.into(),
        }
    }
}

/// 文本样式
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    /// 字体家族
    pub font_family: String,
    /// 字体大小
    pub font_size: f64,
    /// 字体粗细
    pub font_weight: FontWeight,
    /// 字体样式
    pub font_style: FontStyle,
    /// 填充样式
    pub fill: FillStyle,
}

impl TextStyle {
    /// 创建默认文本样式
    pub fn new() -> Self {
        Self {
            font_family: "sans-serif".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            font_style: FontStyle::Normal,
            fill: FillStyle::Color("#000".to_string()),
        }
    }

    /// 设置字体家族
    pub fn with_font_family(mut self, font_family: impl Into<String>) -> Self {
        self.font_family = font_family.into();
        self
    }

    /// 设置字体大小
    pub fn with_font_size(mut self, font_size: f64) -> Self {
        self.font_size = font_size;
        self
    }

    /// 设置字体粗细
    pub fn with_font_weight(mut self, font_weight: FontWeight) -> Self {
        self.font_weight = font_weight;
        self
    }

    /// 设置字体样式
    pub fn with_font_style(mut self, font_style: FontStyle) -> Self {
        self.font_style = font_style;
        self
    }

    /// 设置填充样式
    pub fn with_fill(mut self, fill: FillStyle) -> Self {
        self.fill = fill;
        self
    }

    /// 生成 CSS 字体字符串
    pub fn to_css_font(&self) -> String {
        let style = match self.font_style {
            FontStyle::Normal => "",
            FontStyle::Italic => "italic ",
        };

        let weight = match self.font_weight {
            FontWeight::Normal => "normal ",
            FontWeight::Bold => "bold ",
            FontWeight::Number(n) => {
                // 简单验证字体粗细值
                if n >= 100 && n <= 900 && n % 100 == 0 {
                    &format!("{} ", n)
                } else {
                    "400 "
                }
            }
        };

        format!("{}{}{}px {}", style, weight, self.font_size, self.font_family)
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self::new()
    }
}

/// 字体粗细
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontWeight {
    /// 正常
    Normal,
    /// 粗体
    Bold,
    /// 数字值 (100-900, 100 的倍数)
    Number(u16),
}

/// 字体样式
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontStyle {
    /// 正常
    Normal,
    /// 斜体
    Italic,
}

/// 文本锚点
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAnchor {
    /// 起点（文本对齐到指定位置的左侧）
    Start,
    /// 中间（文本居中对齐到指定位置）
    Middle,
    /// 终点（文本对齐到指定位置的右侧）
    End,
}

impl fmt::Display for TextAnchor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextAnchor::Start => write!(f, "start"),
            TextAnchor::Middle => write!(f, "middle"),
            TextAnchor::End => write!(f, "end"),
        }
    }
}

/// 文本基线
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextBaseline {
    /// 顶部
    Top,
    /// 中间
    Middle,
    /// 底部
    Bottom,
    /// 字母基线（默认）
    Alphabetic,
}

impl fmt::Display for TextBaseline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextBaseline::Top => write!(f, "top"),
            TextBaseline::Middle => write!(f, "middle"),
            TextBaseline::Bottom => write!(f, "bottom"),
            TextBaseline::Alphabetic => write!(f, "alphabetic"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gradient_stop_clamping() {
        let stop1 = GradientStop::new(-0.5, "red");
        assert_eq!(stop1.offset, 0.0);

        let stop2 = GradientStop::new(1.5, "blue");
        assert_eq!(stop2.offset, 1.0);

        let stop3 = GradientStop::new(0.5, "green");
        assert_eq!(stop3.offset, 0.5);
    }

    #[test]
    fn test_text_style_builder() {
        let style = TextStyle::new()
            .with_font_family("Arial")
            .with_font_size(14.0)
            .with_font_weight(FontWeight::Bold)
            .with_font_style(FontStyle::Italic)
            .with_fill(FillStyle::Color("#333".to_string()));

        assert_eq!(style.font_family, "Arial");
        assert_eq!(style.font_size, 14.0);
        assert_eq!(style.font_weight, FontWeight::Bold);
        assert_eq!(style.font_style, FontStyle::Italic);
    }

    #[test]
    fn test_text_style_to_css_font() {
        let style = TextStyle::new();
        assert_eq!(style.to_css_font(), "normal 12px sans-serif");

        let bold_italic = TextStyle::new()
            .with_font_weight(FontWeight::Bold)
            .with_font_style(FontStyle::Italic);
        assert_eq!(bold_italic.to_css_font(), "italic bold 12px sans-serif");

        // 数字字体粗细直接使用数字，不加 "normal" 前缀
        let custom_weight = TextStyle::new()
            .with_font_weight(FontWeight::Number(700))
            .with_font_size(16.0);
        assert_eq!(custom_weight.to_css_font(), "700 16px sans-serif");
    }

    #[test]
    fn test_text_anchor_display() {
        assert_eq!(TextAnchor::Start.to_string(), "start");
        assert_eq!(TextAnchor::Middle.to_string(), "middle");
        assert_eq!(TextAnchor::End.to_string(), "end");
    }

    #[test]
    fn test_text_baseline_display() {
        assert_eq!(TextBaseline::Top.to_string(), "top");
        assert_eq!(TextBaseline::Middle.to_string(), "middle");
        assert_eq!(TextBaseline::Bottom.to_string(), "bottom");
        assert_eq!(TextBaseline::Alphabetic.to_string(), "alphabetic");
    }
}
