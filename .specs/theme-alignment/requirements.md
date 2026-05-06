# Theme Alignment Requirements

## What we need

将 deneb-rs 的 theme 系统和 demo 文本渲染对齐到 mermaid-canvas-rs 的架构模式：

1. **Theme trait 重构** — 语义色槽 + 结构色 + name + base font_size + LayoutConfig
2. **新增 3 个主题** — Forest、Nordic、Cappuccino
3. **LayoutConfig 抽取** — 图表布局参数集中配置
4. **文本渲染切换** — demo 从 fontdue 切换到 ab_glyph，采用相同的 glyph 布局算法

## Input & Output

**Input**: 现有 Theme trait（2 主题，fontdue 渲染，硬编码偏移）
**Output**: 对齐 mermaid 架构的新 Theme trait（5 主题，ab_glyph 渲染，LayoutConfig 驱动布局）

## Success criteria

- [ ] Theme trait 包含 `name()`、`font_size()`、语义色槽、结构色方法
- [ ] 5 个内置主题（Default、Dark、Forest、Nordic、Cappuccino），每个 6 色数据系列 + 独立结构色
- [ ] LayoutConfig 结构体集中管理图表布局参数
- [ ] demo 文本渲染使用 ab_glyph，支持 kerning + outline_glyph draw callback
- [ ] 所有现有测试通过，新增主题和 LayoutConfig 的测试
- [ ] `cargo build --workspace` 和 `cargo test --workspace --exclude deneb-demo` 通过

## Edge cases

- **breaking change**: Theme trait 方法签名变更，所有实现 Theme 的代码需要适配
- **fontdue → ab_glyph**: 字体加载路径、API 差异需处理
- **返回类型**: 部分 Theme 方法从 `String` 改为 `&str`，调用方需调整
- **WASM 兼容**: deneb-demo 的文本渲染不在 WASM 路径中，但 deneb-core 的 DrawCmd 不受影响
