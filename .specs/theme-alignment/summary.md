# Theme Alignment — Feature Summary

## 目标

将 deneb-rs 的 theme 系统和 demo 文本渲染对齐到 mermaid-canvas-rs 的架构模式。

## 完成状态

**6/6 任务完成**，所有 requirements 成功标准达成。

## 变更清单

### Theme Trait 重构

- 新增方法：`name()`, `series_color()`, `grid_color()`, `axis_color()`, `title_color()`, `layout_config()`
- `padding()` → `margin()`（与 mermaid 对齐）
- 返回类型 `String` → `&str`（避免堆分配）
- `palette(n)` 保留为向后兼容 default method，内部用 `series_color(slot)` 实现

### LayoutConfig

- 新增 `LayoutConfig` 结构体：8 字段（axis_label_spacing, label_padding, tick_length, title_spacing, line_height, max_label_width_chars, bar_padding_ratio, point_radius）
- Theme trait `layout_config()` 返回 `LayoutConfig::default()`

### 5 个内置主题

| 主题 | 背景 | 调色板 | 色数 |
|------|------|--------|------|
| Default | `#ffffff` | Category10 | 10 |
| Dark | `#1e1e2e` | Tableau10 | 10 |
| Forest | `#1b2a1b` | 多层绿色 | 10 |
| Nordic | `#f8f9fb` | 冷灰蓝 | 10 |
| Cappuccino | `#faf6f1` | 多色相暖色系 | 10 |

### Chart 适配

- 26 处 API 迁移（padding→margin, .to_string(), title_color() 等）
- 8 处 `theme.tick_size()` → `theme.layout_config().tick_length`
- bar.rs 单系列按 category 分色（`theme.series_color(bar_idx)`），多系列按 series 分色

### Bar Chart 正确性修复

- **Y 轴 domain**：`compute_axis_layout` 新增 `include_zero` 参数，`spec.mark == Mark::Bar` 时强制包含 0
- **Cappuccino 调色板**：从单色相棕/米色系替换为多色相暖色系
- **单系列分色**：bar.rs `render_bars` 单系列按 category index 分色，多系列按 series index 分色

### Demo 文本渲染

- fontdue → ab_glyph（`ab_glyph = "0.2"`）
- 重写 `FontState`：`outline_glyph` + `draw` callback
- 支持 kerning + 亚像素定位

### Demo --theme 参数

- 4 个 demo binary 均支持 `--theme default|dark|forest|nordic|cappuccino`
- 泛型 `render_and_show<T, F>` helper + `parse_theme_name()` 选择具体主题

### 文档更新

- CLAUDE.md 新增"轴域 (Axis Domain)"核心规范（Bar 必须从 0，禁止添加不从 0 的配置选项）

## 文件变更

| 文件 | 变更类型 |
|------|---------|
| `crates/deneb-component/src/theme/mod.rs` | 重构（Theme trait + LayoutConfig + 5 主题 + 测试） |
| `crates/deneb-component/src/chart/bar.rs` | 修改（分色逻辑 + include_zero） |
| `crates/deneb-component/src/chart/{line,scatter,area}.rs` | 修改（API 适配） |
| `crates/deneb-component/src/layout/mod.rs` | 修改（include_zero 参数） |
| `crates/deneb-component/src/lib.rs` | 修改（re-exports） |
| `crates/deneb-demo/src/renderer/text.rs` | 重写（ab_glyph） |
| `crates/deneb-demo/src/lib.rs` | 修改（parse_theme_name + render_and_show） |
| `crates/deneb-demo/src/bin/demo_*.rs` | 修改（--theme 参数） |
| `crates/deneb-demo/Cargo.toml` | 修改（ab_glyph 替换 fontdue） |
| `CLAUDE.md` | 新增（轴域规范） |
| `.specs/project-info.md` | 更新（主题系统、ab_glyph、测试数） |

## 测试

- **259 tests 通过**（100 deneb-component + 134 deneb-core + 20 deneb-wit + 5 doc）
- 新增测试：5 个主题各 4-5 个测试（colors, palette, name, structural_colors）
- `cargo build --workspace --exclude deneb-wit-wasm` ✅
- `cargo test --workspace --exclude deneb-demo` ✅

## 架构决策

1. **`palette(n)` 保留为 default method** — 向后兼容，内部用 `series_color(slot)` 循环
2. **返回 `&str`** — 静态颜色避免堆分配，调用方 `.to_string()` 按需分配
3. **LayoutConfig 独立 struct** — 与 Theme 分离，同一主题可用不同布局
4. **Bar Y 轴 include_zero** — 由 `spec.mark` 驱动，不是用户可配置选项（数据可视化正确性要求）
5. **单系列 bar 按 category 分色** — 符合 ECharts `colorBy: 'data'` 模式

## 已知遗留

- Forest/Nordic 调色板色相接近（已记录 might-it-be.md）
- deneb-wit-wasm 有预存在的 WASM linker 错误（与本次变更无关）
