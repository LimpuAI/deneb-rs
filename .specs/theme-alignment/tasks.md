# Theme Alignment Tasks

## Progress
Goal: 对齐 deneb-rs theme 系统和文本渲染到 mermaid-canvas-rs 架构
Status: 6/6 (100%)
Current: Complete
Next: Done

## Tasks

- [x] 1. **重构 Theme trait** — 新增 name/font_size/series_color/grid_color/axis_color/title_color/layout_config 方法，margin 替代 padding，返回类型 &str 化。更新 DefaultTheme 和 DarkTheme 实现。 — ref: requirements "Theme trait 重构", design "Theme Trait"
- [x] 2. **新增 LayoutConfig** — 在 deneb-component/src/theme/ 中添加 LayoutConfig struct 及 Default 实现。Theme trait 新增 layout_config() 方法。 — ref: requirements "LayoutConfig 抽取", design "LayoutConfig"
- [x] 3. **新增 Forest/Nordic/Cappuccino 主题** — 实现 3 个新主题 struct，每个包含 6 色数据系列色板 + 结构色。编写主题测试。 — ref: requirements "新增 3 个主题", design "5 个内置主题色板"
- [x] 4. **适配 chart 实现** — 更新 line.rs/bar.rs/scatter.rs/area.rs 中所有 Theme 方法调用（padding→margin, foreground_color→title_color 等）。利用 LayoutConfig 替代硬编码偏移。 — ref: requirements "breaking change 适配"
- [x] 5. **demo 文本渲染切换 ab_glyph** — Cargo.toml 替换 fontdue→ab_glyph，重写 text.rs 的 FontState，采用 outline_glyph + draw callback 模式。 — ref: requirements "文本渲染切换", design "ab_glyph 替代 fontdue"
- [x] 6. **验证与测试** — cargo build --workspace，cargo test --workspace --exclude deneb-demo，新增主题/布局测试通过。lsp_diagnostics 清洁。 — ref: requirements "Success criteria"

## Notes

- Task 1-2 可并行（Theme trait + LayoutConfig 定义）
- Task 3 依赖 Task 1（新主题实现新 trait）
- Task 4 依赖 Task 1+2（chart 适配新 trait + LayoutConfig）
- Task 5 独立于 Task 1-4（demo 渲染层变更）
- Task 6 依赖所有其他 task 完成
