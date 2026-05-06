# Theme Alignment Design

## API Contracts

### Theme Trait（重构后）

```rust
/// 主题 trait — 定义图表的视觉风格
pub trait Theme: Clone {
    // ── 身份 ──
    fn name(&self) -> &str;

    // ── 数据系列色（语义色槽）──
    /// 获取数据系列色（slot 0-5）
    fn series_color(&self, slot: usize) -> &str;

    /// 获取 n 个系列色（向后兼容，内部循环 series_color）
    fn palette(&self, n: usize) -> Vec<String>;

    // ── 结构色 ──
    fn background_color(&self) -> &str;
    fn foreground_color(&self) -> &str;
    fn grid_color(&self) -> &str;
    fn axis_color(&self) -> &str;
    fn title_color(&self) -> &str;

    // ── 排版 ──
    fn font_family(&self) -> &str;
    fn font_size(&self) -> f64;           // 基础字号（默认 14.0）
    fn title_font_size(&self) -> f64;     // 默认 font_size() * 1.2
    fn label_font_size(&self) -> f64;     // 默认 font_size() * 0.9
    fn tick_font_size(&self) -> f64;      // 默认 font_size() * 0.75

    // ── 线条 ──
    fn grid_stroke(&self) -> StrokeStyle;
    fn axis_stroke(&self) -> StrokeStyle;
    fn default_stroke_width(&self) -> f64;

    // ── 布局 ──
    fn margin(&self) -> Margin;
    fn tick_size(&self) -> f64;
    fn layout_config(&self) -> LayoutConfig;
}
```

**关键变更**:
- `palette(n)` 保留但内部用 `series_color(slot)` 实现
- 新增 `name()`、`font_size()`、`series_color()`、`grid_color()`、`axis_color()`、`title_color()`
- `background_color()` 和 `foreground_color()` 返回 `&str`（原 `String`）
- `padding()` 重命名为 `margin()`（与 mermaid 对齐）
- 新增 `layout_config()` 返回 `LayoutConfig`

### LayoutConfig（新增）

```rust
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
```

### 5 个内置主题色板

每个主题定义 6 个数据系列色（`PALETTE` 常量）：

| Theme | 背景 | 色槽风格 |
|-------|------|---------|
| Default | `#ffffff` | Category10 经典 |
| Dark | `#1e1e2e` | Tableau10 深色优化 |
| Forest | `#1b2a1b` | 多层次绿色 |
| Nordic | `#f8f9fb` | 冷灰蓝极简 |
| Cappuccino | `#faf6f1` | 暖棕奶咖 |

## Key decisions

- **返回 `&str` 而非 `String`**: 避免每次调用的堆分配，主题颜色是静态的。调用方需要 `.to_string()` 时再分配。
- **`palette(n)` 保留**: 向后兼容。内部实现为 `series_color(i % 6)` 循环，但色槽数组为 10 色以支持更多系列。
- **`font_size()` + 派生字号**: 基础字号 `font_size()` 默认 14.0，`title_font_size()` 等通过默认倍率派生。具体主题可覆盖。
- **LayoutConfig 独立 struct**: 与 Theme 分离，允许同一主题使用不同布局配置。LayoutConfig 实现 Default，Theme 的 `layout_config()` 默认返回 `LayoutConfig::default()`。
- **ab_glyph 替代 fontdue**: ab_glyph 是 rusttype 的继任者，支持 outline_glyph + draw callback，渲染质量更高（亚像素定位、kerning）。

## Integration points

- **deneb-core**: `DrawCmd::Text` 和 `CanvasOp` 不变（已经是语义层）
- **deneb-component**: Theme trait 重构影响所有 chart 实现（line/bar/scatter/area）
- **deneb-demo**: `FontState` 重写（fontdue → ab_glyph），`Cargo.toml` 依赖变更
- **deneb-wit / deneb-wit-wasm**: 不受影响（WASM 路径不经过 demo 渲染）
