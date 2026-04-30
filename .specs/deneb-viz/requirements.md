# deneb-viz Requirements

## What we need
后端无关的 Rust 可视化库，输出 Canvas 2D 指令序列。数据与绘图逻辑分离，API 启发自 Vega-Lite 的绘图语法。为 WebAssembly 一等公民设计，支持 WASI 0.3。宿主负责渲染执行和响应式调度，deneb-rs 提供计算核心。

## Core Characteristics
- **后端无关**: Canvas 2D 指令序列，不绑定任何渲染后端（浏览器 Canvas/vello/Skia/自定义）
- **数据与绘图分离**: 数据解析、编码映射、指令生成三层解耦
- **绘图语法 API**: Builder 模式，类型安全，启发自 Vega-Lite
- **细粒度响应式**: 分层标记 + dirty flag，宿主按需重绘指定层
- **WASM 一等公民**: WASI 0.3 特性，支持库模式和独立组件模式
- **多数据格式**: CSV、JSON、Arrow IPC、Parquet

## Input & Output
**Input**: 数据（CSV/JSON/Arrow/Parquet bytes）+ ChartSpec（声明式配置）
**Output**: Canvas 2D 指令序列（语义枚举 + Canvas 2D API 映射双层），按层组织，带 dirty flag

## Crate Structure
- **deneb-core**: 纯计算核心。数据解析、Scale 计算、编码映射、Canvas 2D 指令枚举定义、降采样算法（LTTB/M4）、分层系统、交互元数据生成
- **deneb-component**: Chart 类型实现层。Line/Bar/Scatter/Area 的布局计算、轴/刻度分布、图例编排、Theme trait、完整图表指令生成
- **deneb-wit**: WASI 0.3 集成。WIT 接口定义、库模式（嵌入调用）、独立组件模式
- **deneb-wit-wasm**: WASI Component Model 导出层。使用 wit-bindgen 0.51 生成 guest 绑定，编译为 wasm32-wasip2 目标
- **deneb-demo**: 桌面演示。tiny-skia 渲染器 + wasmtime WASM Host + winit 窗口

## Success criteria
- [x] deneb-core 零依赖外部渲染库，可独立编译为 wasm32-wasip2
- [x] 同一 ChartSpec + Theme 在任何后端产生一致的 Canvas 2D 指令输出
- [ ] 10K 数据点渲染指令生成 < 5ms（WASM 环境）
- [x] 数据变化时只重标记受影响层，未变化层不重新生成指令
- [x] 支持 CSV、JSON、Arrow IPC、Parquet 四种数据格式输入
- [x] Line / Bar / Scatter / Area 四种 Chart 类型可用
- [x] 交互元数据可导出（数据点包围盒 + 索引），宿主可做命中检测
- [x] Theme trait 可完全替换，适配宿主视觉风格
- [x] WIT 接口同时支持库调用模式和独立组件运行模式
- [x] 可编译为 WASI Component Model 格式，wasmtime 可加载并调用（deneb-wit-wasm）
- [x] 宿主端可通过 wasmtime bindgen 类型安全地调用 WASM 组件（deneb-demo WasmHost）

## Data Scale
- 目标: 10K - 100K 数据点
- 内建降采样: LTTB（时间序列保持视觉特征）+ M4（快速 min-max 降采样）
- 自动触发阈值可配置

## Interactivity Design
- deneb-core 定义交互接口 trait（HitRegion / HoverMetadata / SelectionMetadata）
- 生成 Canvas 2D 指令时同时生成交互元数据（数据点坐标 + 包围盒 + 索引）
- 事件分发和 UI 反馈由宿主负责
- deneb-core 提供坐标反查接口（像素坐标 → 数据点索引）

## Edge cases
- **空数据**: 返回空指令序列 + Empty 状态标记，不 panic
- **单数据点**: Line/Area 退化为点标记，Bar 退化为单柱
- **全相同值**: Scale 退化为常数映射，轴显示单一刻度
- **混合数据类型**: 类型推断优先级：Numeric > Timestamp > Text > Null
- **超大数据集**: 超过阈值自动降采样，输出降采样元数据供宿主提示用户
- **无效配置**: 编译期通过类型系统阻止（如 Line chart 缺少 x/y encoding）

## Open decisions
- [ ] JSON 数据输入的具体 schema 约定（flat array vs nested object）
- [ ] Canvas 2D API 映射层的具体指令集范围
- [ ] 降采样自动触发的默认阈值
