# deneb-viz Feature Summary

## 目标
构建后端无关的 Canvas 2D 指令序列可视化库，为 WASI Component Model 设计。

## 完成状态
27/27 任务完成，248 单元测试全部通过，cc-review 审计通过（P1/P2 已修正）。

## 交付物

### Crate 结构

| Crate | 职责 | 行数 | 测试 |
|-------|------|------|------|
| deneb-core | 数据类型、绘图指令、Scale、解析器、降采样、交互 | 5934 | 134 + 5 doc |
| deneb-component | Line/Bar/Scatter/Area 图表、布局引擎、Theme | 5122 | 76 |
| deneb-wit | WIT 类型转换、lib_mode API、独立组件模式 | 1037 | 20 |
| deneb-wit-wasm | WASI Component Model 导出（wit-bindgen 0.51） | 160 | — |
| deneb-demo | TinySkiaRenderer + WasmHost + 4 demo binary | 1576 | 13 |

**总计**：13,829 行 Rust 代码，248 测试

### 功能覆盖

**数据格式**：CSV、JSON、Arrow IPC、Parquet（后两者为可选 feature）

**图表类型**：Line（多系列）、Bar（band scale）、Scatter（大小/颜色映射）、Area（堆叠）

**Scale 系统**：Linear、Log、Ordinal、OrdinalBand、Band、Time — 统一 Scale trait

**分层渲染**：7 层（Background → Grid → Axis → Data → Legend → Title → Annotation），支持 dirty flag 增量标记

**交互**：HitRegion + BoundingBox + 坐标反查（像素 → 数据索引）

**WASI**：WIT 接口（data-parser + chart-renderer）、库调用模式、独立组件模式、wasm32-wasip2 目标编译（498KB release）

**Demo**：4 个桌面 demo binary（winit + softbuffer），支持 Native/WASM 双路径切换（--wasm 参数）

### 类型编码策略（WASM 边界）

| 内部类型 | WIT 传输 | 编码方式 |
|---------|---------|---------|
| DrawCmd 枚举 | cmd_type + params | 字符串标记 + f64 数组 |
| Group 递归 | group_depth: u32 | 线性展平 |
| PathSegment 枚举 | params 前缀 | 0=MoveTo, 1=LineTo, ... |
| TextAnchor/Baseline | params 数字 | 0/1/2/3 编码 |
| DataTable 列式 | 行式传输 | 行列转置 |

### 文档

- docs/（Mint 框架）：architecture、line-chart、bar-chart、scatter-chart、area-chart、webassembly、demo
- .specs/deneb-viz/：requirements.md、design.md、tasks.md

## 设计-实现偏差（已同步）

10 处设计文档与实际实现的不一致，全部已更新至 design.md。核心偏差：
- WIT 接口方向：import → export
- 递归类型：children → group_depth 展平
- 5 crate 架构（原设计 3 crate）

## cc-review 审计结果

| 维度 | 状态 | 问题 |
|------|------|------|
| P0 功能合规 | PASS | 0 |
| P1 架构合规 | PASS | 0（已修正） |
| P2 代码质量 | PASS | 0（已修正） |

修正记录：
- P1：DataTable 新增 schema 字段，O(n) → O(1) 列查找
- P2：移除 BandScale 未使用 index_of 方法

## 开发时间线

- 2026-04-29：Phase 1-4 完成（core + component，通过 cc-wave 并行执行）
- 2026-04-30：Phase 5 + Phase B 完成（WIT + WASM Component + Demo）
- 2026-04-30：文档初始化、cc-sync、design.md 同步、cc-review 审计、P1/P2 修正

## 遗留项

参见 .specs/might-it-be.md：
- 增量更新 update-data（推迟）
- Canvas 2D 完整指令集（推迟）
- 降采样自动触发阈值（Open decision）
- JSON Schema 约定（Open decision）
- 更多图表类型（扩展点）
- 响应式集成方案（宿主侧设计）
