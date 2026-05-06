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

/// 图表布局配置
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutConfig {
    /// 轴线与刻度标签间距
    pub axis_label_spacing: f64,
    /// 标签与轴线内边距
    pub label_padding: f64,
    /// 刻度线长度
    pub tick_length: f64,
    /// 标题与绘图区间距
    pub title_spacing: f64,
    /// 文本行高倍数（默认 1.4）
    pub line_height: f64,
    /// 刻度标签最大字符宽度
    pub max_label_width_chars: usize,
    /// 条形图间距比例（0.0-1.0）
    pub bar_padding_ratio: f64,
    /// 散点图默认点大小
    pub point_radius: f64,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            axis_label_spacing: 8.0,
            label_padding: 4.0,
            tick_length: 6.0,
            title_spacing: 10.0,
            line_height: 1.4,
            max_label_width_chars: 20,
            bar_padding_ratio: 0.2,
            point_radius: 4.0,
        }
    }
}

/// 主题 trait — 定义图表的视觉风格
pub trait Theme: Clone {
    /// 主题名称
    fn name(&self) -> &str;

    /// 获取系列颜色（按 slot 索引）
    fn series_color(&self, slot: usize) -> &str;

    /// 获取调色板（返回 n 个颜色）
    fn palette(&self, n: usize) -> Vec<String> {
        (0..n).map(|i| self.series_color(i).to_string()).collect()
    }

    /// 获取背景颜色
    fn background_color(&self) -> &str;

    /// 获取前景颜色（文本、轴线等）
    fn foreground_color(&self) -> &str;

    /// 获取字体家族
    fn font_family(&self) -> &str;

    /// 基础字体大小
    fn font_size(&self) -> f64 {
        14.0
    }

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

    /// 获取边距
    fn margin(&self) -> Margin;

    /// 获取刻度大小
    fn tick_size(&self) -> f64;

    /// 网格颜色
    fn grid_color(&self) -> &str;

    /// 轴线颜色
    fn axis_color(&self) -> &str;

    /// 标题颜色
    fn title_color(&self) -> &str;

    /// 布局配置
    fn layout_config(&self) -> LayoutConfig {
        LayoutConfig::default()
    }
}

/// DefaultTheme — 浅色主题
#[derive(Clone, Debug)]
pub struct DefaultTheme;

impl DefaultTheme {
    /// Category10 颜色方案
    const PALETTE: [&'static str; 10] = [
        "#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd",
        "#8c564b", "#e377c2", "#7f7f7f", "#bcbd22", "#17becf",
    ];
}

impl Theme for DefaultTheme {
    fn name(&self) -> &str {
        "Default"
    }

    fn series_color(&self, slot: usize) -> &str {
        Self::PALETTE[slot % Self::PALETTE.len()]
    }

    fn background_color(&self) -> &str {
        "#ffffff"
    }

    fn foreground_color(&self) -> &str {
        "#333333"
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

    fn margin(&self) -> Margin {
        Margin::new(30.0, 20.0, 40.0, 50.0)
    }

    fn tick_size(&self) -> f64 {
        6.0
    }

    fn grid_color(&self) -> &str {
        "#e0e0e0"
    }

    fn axis_color(&self) -> &str {
        "#333333"
    }

    fn title_color(&self) -> &str {
        "#333333"
    }
}

/// DarkTheme — 深色主题
#[derive(Clone, Debug)]
pub struct DarkTheme;

impl DarkTheme {
    /// Tableau10 颜色方案（深色背景下更醒目）
    const PALETTE: [&'static str; 10] = [
        "#4e79a7", "#f28e2b", "#e15759", "#76b7b2", "#59a14f",
        "#edc948", "#b07aa1", "#ff9da7", "#9c755f", "#bab0ac",
    ];
}

impl Theme for DarkTheme {
    fn name(&self) -> &str {
        "Dark"
    }

    fn series_color(&self, slot: usize) -> &str {
        Self::PALETTE[slot % Self::PALETTE.len()]
    }

    fn background_color(&self) -> &str {
        "#1e1e2e"
    }

    fn foreground_color(&self) -> &str {
        "#cdd6f4"
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

    fn margin(&self) -> Margin {
        Margin::new(30.0, 20.0, 40.0, 50.0)
    }

    fn tick_size(&self) -> f64 {
        6.0
    }

    fn grid_color(&self) -> &str {
        "#45475a"
    }

    fn axis_color(&self) -> &str {
        "#cdd6f4"
    }

    fn title_color(&self) -> &str {
        "#cdd6f4"
    }
}

/// ForestTheme — 森林主题，深绿色背景 + 多层绿色调
#[derive(Clone, Debug)]
pub struct ForestTheme;

impl ForestTheme {
    const PALETTE: [&'static str; 10] = [
        "#2d5a27", "#3a6b34", "#4a7c3f", "#1e4d2b", "#5a3a27",
        "#3a6b34", "#8bc34a", "#689f38", "#558b2f", "#33691e",
    ];
}

impl Theme for ForestTheme {
    fn name(&self) -> &str {
        "Forest"
    }

    fn series_color(&self, slot: usize) -> &str {
        Self::PALETTE[slot % Self::PALETTE.len()]
    }

    fn background_color(&self) -> &str {
        "#1b2a1b"
    }

    fn foreground_color(&self) -> &str {
        "#e8f5e9"
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
        StrokeStyle::Color("#2d5a27".to_string())
    }

    fn axis_stroke(&self) -> StrokeStyle {
        StrokeStyle::Color("#8bc34a".to_string())
    }

    fn default_stroke_width(&self) -> f64 {
        1.0
    }

    fn margin(&self) -> Margin {
        Margin::new(30.0, 20.0, 40.0, 50.0)
    }

    fn tick_size(&self) -> f64 {
        6.0
    }

    fn grid_color(&self) -> &str {
        "#2d5a27"
    }

    fn axis_color(&self) -> &str {
        "#8bc34a"
    }

    fn title_color(&self) -> &str {
        "#c5e1a5"
    }
}

/// NordicTheme — 北欧主题，冷灰蓝 + 浅粉点缀
#[derive(Clone, Debug)]
pub struct NordicTheme;

impl NordicTheme {
    const PALETTE: [&'static str; 10] = [
        "#5b8db8", "#7ba3c7", "#a3c4d9", "#d4a0a0", "#8fb3c9",
        "#6a9fc4", "#93b8d4", "#c4908f", "#7baec2", "#a8c5d8",
    ];
}

impl Theme for NordicTheme {
    fn name(&self) -> &str {
        "Nordic"
    }

    fn series_color(&self, slot: usize) -> &str {
        Self::PALETTE[slot % Self::PALETTE.len()]
    }

    fn background_color(&self) -> &str {
        "#f8f9fb"
    }

    fn foreground_color(&self) -> &str {
        "#3d4f5f"
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
        StrokeStyle::Color("#dfe6ed".to_string())
    }

    fn axis_stroke(&self) -> StrokeStyle {
        StrokeStyle::Color("#8b9eb0".to_string())
    }

    fn default_stroke_width(&self) -> f64 {
        1.0
    }

    fn margin(&self) -> Margin {
        Margin::new(30.0, 20.0, 40.0, 50.0)
    }

    fn tick_size(&self) -> f64 {
        6.0
    }

    fn grid_color(&self) -> &str {
        "#dfe6ed"
    }

    fn axis_color(&self) -> &str {
        "#8b9eb0"
    }

    fn title_color(&self) -> &str {
        "#3d4f5f"
    }
}

/// CappuccinoTheme — 卡布奇诺主题，温暖棕咖啡色调
#[derive(Clone, Debug)]
pub struct CappuccinoTheme;

impl CappuccinoTheme {
    const PALETTE: [&'static str; 10] = [
        "#5d3a1a", // 深浓缩咖啡
        "#c25e3f", // 陶土橙
        "#d4a843", // 琥珀金
        "#b87b8a", // 玫瑰灰
        "#7a8b5c", // 橄榄绿
        "#c7793e", // 暖铜色
        "#9c6b4e", // 焦赭
        "#6b9b8a", // 青绿
        "#8b4e5c", // 酒红
        "#c9a87c", // 驼色
    ];
}

impl Theme for CappuccinoTheme {
    fn name(&self) -> &str {
        "Cappuccino"
    }

    fn series_color(&self, slot: usize) -> &str {
        Self::PALETTE[slot % Self::PALETTE.len()]
    }

    fn background_color(&self) -> &str {
        "#faf6f1"
    }

    fn foreground_color(&self) -> &str {
        "#3e2c1c"
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
        StrokeStyle::Color("#e0ddd8".to_string())
    }

    fn axis_stroke(&self) -> StrokeStyle {
        StrokeStyle::Color("#8b6f4e".to_string())
    }

    fn default_stroke_width(&self) -> f64 {
        1.0
    }

    fn margin(&self) -> Margin {
        Margin::new(30.0, 20.0, 40.0, 50.0)
    }

    fn tick_size(&self) -> f64 {
        6.0
    }

    fn grid_color(&self) -> &str {
        "#e0ddd8"
    }

    fn axis_color(&self) -> &str {
        "#8b6f4e"
    }

    fn title_color(&self) -> &str {
        "#5d3a1a"
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
    fn test_default_theme_margin() {
        let theme = DefaultTheme;
        let margin = theme.margin();
        assert_eq!(margin, Margin::new(30.0, 20.0, 40.0, 50.0));
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

    // --- 新增测试：name, series_color, font_size, grid_color, axis_color, title_color ---

    #[test]
    fn test_default_theme_name() {
        let theme = DefaultTheme;
        assert_eq!(theme.name(), "Default");
    }

    #[test]
    fn test_dark_theme_name() {
        let theme = DarkTheme;
        assert_eq!(theme.name(), "Dark");
    }

    #[test]
    fn test_series_color() {
        let theme = DefaultTheme;
        assert_eq!(theme.series_color(0), "#1f77b4");
        assert_eq!(theme.series_color(1), "#ff7f0e");
        // 循环
        assert_eq!(theme.series_color(10), "#1f77b4");
        assert_eq!(theme.series_color(11), "#ff7f0e");
    }

    #[test]
    fn test_dark_series_color() {
        let theme = DarkTheme;
        assert_eq!(theme.series_color(0), "#4e79a7");
        assert_eq!(theme.series_color(1), "#f28e2b");
        // 循环
        assert_eq!(theme.series_color(10), "#4e79a7");
    }

    #[test]
    fn test_font_size() {
        let theme = DefaultTheme;
        assert_eq!(theme.font_size(), 14.0);

        let dark = DarkTheme;
        assert_eq!(dark.font_size(), 14.0);
    }

    #[test]
    fn test_grid_color() {
        let theme = DefaultTheme;
        assert_eq!(theme.grid_color(), "#e0e0e0");

        let dark = DarkTheme;
        assert_eq!(dark.grid_color(), "#45475a");
    }

    #[test]
    fn test_axis_color() {
        let theme = DefaultTheme;
        assert_eq!(theme.axis_color(), "#333333");

        let dark = DarkTheme;
        assert_eq!(dark.axis_color(), "#cdd6f4");
    }

    #[test]
    fn test_title_color() {
        let theme = DefaultTheme;
        assert_eq!(theme.title_color(), "#333333");

        let dark = DarkTheme;
        assert_eq!(dark.title_color(), "#cdd6f4");
    }

    #[test]
    fn test_layout_config_default() {
        let config = LayoutConfig::default();
        assert_eq!(config.axis_label_spacing, 8.0);
        assert_eq!(config.label_padding, 4.0);
        assert_eq!(config.tick_length, 6.0);
        assert_eq!(config.title_spacing, 10.0);
        assert_eq!(config.line_height, 1.4);
        assert_eq!(config.max_label_width_chars, 20);
        assert_eq!(config.bar_padding_ratio, 0.2);
        assert_eq!(config.point_radius, 4.0);
    }

    #[test]
    fn test_theme_layout_config() {
        let theme = DefaultTheme;
        let config = theme.layout_config();
        assert_eq!(config, LayoutConfig::default());
    }

    #[test]
    fn test_dark_theme_margin() {
        let theme = DarkTheme;
        let margin = theme.margin();
        assert_eq!(margin, Margin::new(30.0, 20.0, 40.0, 50.0));
    }

    // --- ForestTheme 测试 ---

    #[test]
    fn test_forest_theme_colors() {
        let theme = ForestTheme;
        assert_eq!(theme.background_color(), "#1b2a1b");
        assert_eq!(theme.foreground_color(), "#e8f5e9");
    }

    #[test]
    fn test_forest_theme_palette() {
        let theme = ForestTheme;
        let palette5 = theme.palette(5);
        assert_eq!(palette5.len(), 5);
        assert_eq!(palette5[0], "#2d5a27");
        assert_eq!(palette5[1], "#3a6b34");
        assert_eq!(palette5[2], "#4a7c3f");
        assert_eq!(palette5[3], "#1e4d2b");
        assert_eq!(palette5[4], "#5a3a27");
    }

    #[test]
    fn test_forest_theme_name() {
        let theme = ForestTheme;
        assert_eq!(theme.name(), "Forest");
    }

    #[test]
    fn test_forest_theme_structural_colors() {
        let theme = ForestTheme;
        assert_eq!(theme.grid_color(), "#2d5a27");
        assert_eq!(theme.axis_color(), "#8bc34a");
        assert_eq!(theme.title_color(), "#c5e1a5");
    }

    // --- NordicTheme 测试 ---

    #[test]
    fn test_nordic_theme_colors() {
        let theme = NordicTheme;
        assert_eq!(theme.background_color(), "#f8f9fb");
        assert_eq!(theme.foreground_color(), "#3d4f5f");
    }

    #[test]
    fn test_nordic_theme_palette() {
        let theme = NordicTheme;
        let palette5 = theme.palette(5);
        assert_eq!(palette5.len(), 5);
        assert_eq!(palette5[0], "#5b8db8");
        assert_eq!(palette5[1], "#7ba3c7");
        assert_eq!(palette5[2], "#a3c4d9");
        assert_eq!(palette5[3], "#d4a0a0");
        assert_eq!(palette5[4], "#8fb3c9");
    }

    #[test]
    fn test_nordic_theme_name() {
        let theme = NordicTheme;
        assert_eq!(theme.name(), "Nordic");
    }

    #[test]
    fn test_nordic_theme_structural_colors() {
        let theme = NordicTheme;
        assert_eq!(theme.grid_color(), "#dfe6ed");
        assert_eq!(theme.axis_color(), "#8b9eb0");
        assert_eq!(theme.title_color(), "#3d4f5f");
    }

    // --- CappuccinoTheme 测试 ---

    #[test]
    fn test_cappuccino_theme_colors() {
        let theme = CappuccinoTheme;
        assert_eq!(theme.background_color(), "#faf6f1");
        assert_eq!(theme.foreground_color(), "#3e2c1c");
    }

    #[test]
    fn test_cappuccino_theme_palette() {
        let theme = CappuccinoTheme;
        let palette5 = theme.palette(5);
        assert_eq!(palette5.len(), 5);
        assert_eq!(palette5[0], "#5d3a1a");
        assert_eq!(palette5[1], "#c25e3f");
        assert_eq!(palette5[2], "#d4a843");
        assert_eq!(palette5[3], "#b87b8a");
        assert_eq!(palette5[4], "#7a8b5c");
    }

    #[test]
    fn test_cappuccino_theme_name() {
        let theme = CappuccinoTheme;
        assert_eq!(theme.name(), "Cappuccino");
    }

    #[test]
    fn test_cappuccino_theme_structural_colors() {
        let theme = CappuccinoTheme;
        assert_eq!(theme.grid_color(), "#e0ddd8");
        assert_eq!(theme.axis_color(), "#8b6f4e");
        assert_eq!(theme.title_color(), "#5d3a1a");
    }

    // --- 全主题测试 ---

    #[test]
    fn test_all_themes_names() {
        let names: Vec<&str> = vec![
            DefaultTheme.name(),
            DarkTheme.name(),
            ForestTheme.name(),
            NordicTheme.name(),
            CappuccinoTheme.name(),
        ];
        assert_eq!(names, vec!["Default", "Dark", "Forest", "Nordic", "Cappuccino"]);
    }
}
