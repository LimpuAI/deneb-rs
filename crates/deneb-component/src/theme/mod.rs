//! Theme trait 和预置主题
//!
//! 定义图表的视觉风格。

use deneb_core::StrokeStyle;

/// 边距
#[derive(Clone, Debug, PartialEq)]
pub struct Margin {
    /// 上边距
    pub top: f64,
    /// 右边距
    pub right: f64,
    /// 下边距
    pub bottom: f64,
    /// 左边距
    pub left: f64,
}

impl Margin {
    /// 创建新的边距
    pub fn new(top: f64, right: f64, bottom: f64, left: f64) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// 创建统一边距
    pub fn uniform(value: f64) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// 获取水平边距总和
    pub fn horizontal(&self) -> f64 {
        self.left + self.right
    }

    /// 获取垂直边距总和
    pub fn vertical(&self) -> f64 {
        self.top + self.bottom
    }
}

/// 主题 trait — 定义图表的视觉风格
pub trait Theme: Clone {
    /// 获取调色板（返回 n 个颜色）
    fn palette(&self, n: usize) -> Vec<String>;

    /// 获取背景颜色
    fn background_color(&self) -> String;

    /// 获取前景颜色（文本、轴线等）
    fn foreground_color(&self) -> String;

    /// 获取字体家族
    fn font_family(&self) -> &str;

    /// 获取标题字体大小
    fn title_font_size(&self) -> f64;

    /// 获取标签字体大小
    fn label_font_size(&self) -> f64;

    /// 获取刻度字体大小
    fn tick_font_size(&self) -> f64;

    /// 获取网格线样式
    fn grid_stroke(&self) -> StrokeStyle;

    /// 获取轴线样式
    fn axis_stroke(&self) -> StrokeStyle;

    /// 获取默认线宽
    fn default_stroke_width(&self) -> f64;

    /// 获取内边距
    fn padding(&self) -> Margin;

    /// 获取刻度大小
    fn tick_size(&self) -> f64;
}

/// D3 Category10 颜色方案
fn category10_colors() -> &'static [&'static str] {
    &[
        "#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd",
        "#8c564b", "#e377c2", "#7f7f7f", "#bcbd22", "#17becf",
    ]
}

/// Tableau10 颜色方案（用于深色主题）
fn tableau10_colors() -> &'static [&'static str] {
    &[
        "#4e79a7", "#f28e2b", "#e15759", "#76b7b2", "#59a14f",
        "#edc948", "#b07aa1", "#ff9da7", "#9c755f", "#bab0ac",
    ]
}

/// DefaultTheme — 浅色主题
#[derive(Clone, Debug)]
pub struct DefaultTheme;

impl Theme for DefaultTheme {
    fn palette(&self, n: usize) -> Vec<String> {
        let colors = category10_colors();
        let mut result = Vec::with_capacity(n);
        for i in 0..n {
            result.push(colors[i % colors.len()].to_string());
        }
        result
    }

    fn background_color(&self) -> String {
        "#ffffff".to_string()
    }

    fn foreground_color(&self) -> String {
        "#333333".to_string()
    }

    fn font_family(&self) -> &str {
        "sans-serif"
    }

    fn title_font_size(&self) -> f64 {
        16.0
    }

    fn label_font_size(&self) -> f64 {
        12.0
    }

    fn tick_font_size(&self) -> f64 {
        10.0
    }

    fn grid_stroke(&self) -> StrokeStyle {
        StrokeStyle::Color("#e0e0e0".to_string())
    }

    fn axis_stroke(&self) -> StrokeStyle {
        StrokeStyle::Color("#333333".to_string())
    }

    fn default_stroke_width(&self) -> f64 {
        1.0
    }

    fn padding(&self) -> Margin {
        Margin::new(30.0, 20.0, 40.0, 50.0)
    }

    fn tick_size(&self) -> f64 {
        6.0
    }
}

/// DarkTheme — 深色主题
#[derive(Clone, Debug)]
pub struct DarkTheme;

impl Theme for DarkTheme {
    fn palette(&self, n: usize) -> Vec<String> {
        // 使用 Tableau10，在深色背景下更醒目
        let colors = tableau10_colors();
        let mut result = Vec::with_capacity(n);
        for i in 0..n {
            result.push(colors[i % colors.len()].to_string());
        }
        result
    }

    fn background_color(&self) -> String {
        "#1e1e2e".to_string()
    }

    fn foreground_color(&self) -> String {
        "#cdd6f4".to_string()
    }

    fn font_family(&self) -> &str {
        "sans-serif"
    }

    fn title_font_size(&self) -> f64 {
        16.0
    }

    fn label_font_size(&self) -> f64 {
        12.0
    }

    fn tick_font_size(&self) -> f64 {
        10.0
    }

    fn grid_stroke(&self) -> StrokeStyle {
        StrokeStyle::Color("#45475a".to_string())
    }

    fn axis_stroke(&self) -> StrokeStyle {
        StrokeStyle::Color("#cdd6f4".to_string())
    }

    fn default_stroke_width(&self) -> f64 {
        1.0
    }

    fn padding(&self) -> Margin {
        Margin::new(30.0, 20.0, 40.0, 50.0)
    }

    fn tick_size(&self) -> f64 {
        6.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_margin_constructors() {
        let margin1 = Margin::new(10.0, 20.0, 30.0, 40.0);
        assert_eq!(margin1.top, 10.0);
        assert_eq!(margin1.right, 20.0);
        assert_eq!(margin1.bottom, 30.0);
        assert_eq!(margin1.left, 40.0);

        let margin2 = Margin::uniform(15.0);
        assert_eq!(margin2.top, 15.0);
        assert_eq!(margin2.right, 15.0);
        assert_eq!(margin2.bottom, 15.0);
        assert_eq!(margin2.left, 15.0);
    }

    #[test]
    fn test_margin_helpers() {
        let margin = Margin::new(10.0, 20.0, 30.0, 40.0);
        assert_eq!(margin.horizontal(), 60.0);
        assert_eq!(margin.vertical(), 40.0);
    }

    #[test]
    fn test_default_theme_colors() {
        let theme = DefaultTheme;
        assert_eq!(theme.background_color(), "#ffffff");
        assert_eq!(theme.foreground_color(), "#333333");
    }

    #[test]
    fn test_default_theme_fonts() {
        let theme = DefaultTheme;
        assert_eq!(theme.font_family(), "sans-serif");
        assert_eq!(theme.title_font_size(), 16.0);
        assert_eq!(theme.label_font_size(), 12.0);
        assert_eq!(theme.tick_font_size(), 10.0);
    }

    #[test]
    fn test_default_theme_strokes() {
        let theme = DefaultTheme;
        assert_eq!(theme.grid_stroke(), StrokeStyle::Color("#e0e0e0".to_string()));
        assert_eq!(theme.axis_stroke(), StrokeStyle::Color("#333333".to_string()));
        assert_eq!(theme.default_stroke_width(), 1.0);
    }

    #[test]
    fn test_default_theme_padding() {
        let theme = DefaultTheme;
        let padding = theme.padding();
        assert_eq!(padding, Margin::new(30.0, 20.0, 40.0, 50.0));
        assert_eq!(theme.tick_size(), 6.0);
    }

    #[test]
    fn test_default_theme_palette() {
        let theme = DefaultTheme;

        // 请求 5 个颜色
        let palette5 = theme.palette(5);
        assert_eq!(palette5.len(), 5);
        assert_eq!(palette5[0], "#1f77b4");
        assert_eq!(palette5[1], "#ff7f0e");
        assert_eq!(palette5[2], "#2ca02c");
        assert_eq!(palette5[3], "#d62728");
        assert_eq!(palette5[4], "#9467bd");

        // 请求超过 10 个颜色，应该循环
        let palette15 = theme.palette(15);
        assert_eq!(palette15.len(), 15);
        assert_eq!(palette15[0], palette15[10]); // 第 11 个颜色应该和第 1 个相同
    }

    #[test]
    fn test_dark_theme_colors() {
        let theme = DarkTheme;
        assert_eq!(theme.background_color(), "#1e1e2e");
        assert_eq!(theme.foreground_color(), "#cdd6f4");
    }

    #[test]
    fn test_dark_theme_strokes() {
        let theme = DarkTheme;
        assert_eq!(theme.grid_stroke(), StrokeStyle::Color("#45475a".to_string()));
        assert_eq!(theme.axis_stroke(), StrokeStyle::Color("#cdd6f4".to_string()));
    }

    #[test]
    fn test_dark_theme_palette() {
        let theme = DarkTheme;

        // DarkTheme 使用 Tableau10
        let palette5 = theme.palette(5);
        assert_eq!(palette5.len(), 5);
        assert_eq!(palette5[0], "#4e79a7");
        assert_eq!(palette5[1], "#f28e2b");
        assert_eq!(palette5[2], "#e15759");
        assert_eq!(palette5[3], "#76b7b2");
        assert_eq!(palette5[4], "#59a14f");
    }

    #[test]
    fn test_theme_clone() {
        let theme1 = DefaultTheme;
        let theme2 = theme1.clone();
        assert_eq!(theme1.background_color(), theme2.background_color());
    }

    #[test]
    fn test_empty_palette() {
        let theme = DefaultTheme;
        let palette = theme.palette(0);
        assert_eq!(palette.len(), 0);
    }

    #[test]
    fn test_large_palette() {
        let theme = DefaultTheme;
        let palette = theme.palette(25);
        assert_eq!(palette.len(), 25);
        // 验证循环
        assert_eq!(palette[0], palette[10]);
        assert_eq!(palette[1], palette[11]);
    }
}
