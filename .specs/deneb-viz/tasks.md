# deneb-viz Tasks

## Progress
Goal: 构建后端无关的 Canvas 2D 指令序列可视化库
Status: 27/27 (100%)
Current: All tasks completed (包括 Phase B 扩展)
Next: cc-review for audit

## Phase 1: Project Scaffold & Core Types (deneb-core) ✅
- [x] 1. 初始化 workspace + crate 骨架 — ref: requirements Crate Structure
- [x] 2. 定义 deneb-core 基础类型: FieldValue, DataType, Column, DataTable — ref: design 2.1
- [x] 3. 定义 Canvas 2D 指令类型: DrawCmd, PathSegment, CanvasOp, RenderOutput — ref: design 2.2
- [x] 4. 定义 Style 类型: FillStyle, StrokeStyle, TextStyle, TextAnchor, TextBaseline — ref: design 2.2
- [x] 5. 实现 Layer 系统: LayerKind, Layer, RenderLayers, dirty flag 机制 — ref: design 2.3
- [x] 6. 实现 Scale 系统 trait 和具体类型: Linear/Ordinal/Time/Log/Band Scale — ref: design 2.5
- [x] 7. 定义 CoreError 错误类型 — ref: design 6

## Phase 2: Data Parsing (deneb-core) ✅
- [x] 8. 实现 CSV 解析器 (类型推断、日期检测) — ref: requirements 数据格式
- [x] 9. 实现 JSON 解析器 (schema 推断) — ref: requirements 数据格式
- [x] 10. 实现 Arrow IPC 解析器 — ref: requirements 数据格式
- [x] 11. 实现 Parquet 解析器 — ref: requirements 数据格式
- [x] 12. 实现降采样算法: LTTB + M4 — ref: requirements Data Scale

## Phase 3: Interaction Support (deneb-core) ✅
- [x] 13. 实现 HitRegion, BoundingBox, HitResult 类型 — ref: design 2.4
- [x] 14. 实现 CoordLookup trait 和 hit_test/invert 方法 — ref: design 2.4

## Phase 4: Component Layer (deneb-component) ✅
- [x] 15. 定义 ChartSpec builder API: Mark, Field, Encoding, Aggregate — ref: design 2.6
- [x] 16. 定义 Theme trait 和预置主题 (Default, Dark) — ref: design 2.7
- [x] 17. 实现通用布局引擎: margin 计算、轴位置、刻度分布 — ref: design 布局引擎
- [x] 18. 实现 LineChart: 数据映射 + 指令生成 + 交互元数据 — ref: requirements Chart 类型
- [x] 19. 实现 BarChart: band scale + 柱状布局 + 指令生成 — ref: requirements Chart 类型
- [x] 20. 实现 ScatterChart: 散点布局 + 大小映射 + 指令生成 — ref: requirements Chart 类型
- [x] 21. 实现 AreaChart: 面积填充 + 堆叠逻辑 + 指令生成 — ref: requirements Chart 类型

## Phase 5: WIT Integration (deneb-wit) ✅
- [x] 22. 定义 WIT 接口: data-parser + chart-renderer — ref: design 4
- [x] 23. 实现库调用模式 (宿主嵌入调用) — ref: requirements WASI
- [x] 24. 实现独立组件模式 (WASI 命令行运行) — ref: requirements WASI

## Phase B: WASI Component Model + Demo (扩展) ✅
- [x] 25. 实现 deneb-wit-wasm WASI Component 导出层 (wit-bindgen 0.51)
- [x] 26. 实现 deneb-demo 渲染器 (tiny-skia + fontdue + winit) 和 WASM Host (wasmtime 44)
- [x] 27. 修复 WIT 类型编码 (Path 序列化、Text anchor/baseline、Group 展平)

## Notes
- Phase 1-3 (deneb-core) 可独立开发和测试，无外部依赖
- Phase 4 依赖 Phase 1-3 的类型定义
- Phase 5 依赖 Phase 4 的完整实现
- Phase B 是原始 spec 之外的扩展，增加了 deneb-wit-wasm 和 deneb-demo 两个 crate
- 248 单元测试全部通过 (deneb-core: 134, deneb-component: 76, deneb-demo: 13, deneb-wit: 20, doc-tests: 5)

## Design-Implementation Mismatches (已同步至 design.md)

| # | 设计文档 | 实际实现 | 状态 |
|---|---------|---------|------|
| 1 | WIT world `import` 接口 | 实际 `export` 接口 | design.md 已更新 |
| 2 | `draw-cmd` 有 `children` 递归 | 实际用 `group-depth: u32` 展平 | design.md 已更新 |
| 3 | `chart-spec.width/height: u32` | 实际为 `f64` | design.md 已更新 |
| 4 | `render-result` 有 `canvas-ops: list<u8>` | 实际无此字段 | design.md 已更新 |
| 5 | `update-data` 函数 | 未实现（增量更新推迟） | design.md 已更新 |
| 6 | ChartSpec builder API | 实际用 `.encoding(Encoding::new().x().y())` | design.md 已更新 |
| 7 | Encoding 有 `shape` 通道 | 实际仅有 x/y/color/size | design.md 已更新 |
| 8 | `Scale::domain()/range()` 返回引用 | 实际返回 `(f64, f64)` | design.md 已更新 |
| 9 | ComponentError 多变体 | 实际仅有 Core + InvalidConfig | design.md 已更新 |
| 10 | 3 个 crate | 实际 5 个 crate | design.md 已更新 |

## cc-review 修正记录

| 优先级 | 问题 | 状态 |
|--------|------|------|
| P1 | DataTable 缺少 schema 字段 | 已修正：DataTable 新增 `schema: Schema` 字段，自动维护索引 |
| P2 | BandScale::index_of() dead code | 已修正：移除未使用方法 |
