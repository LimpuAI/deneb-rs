# deneb-rs Project Info

## 项目概述
后端无关的 Rust 可视化库，输出 Canvas 2D 指令序列，为 WASI Component Model 设计。

## Crate 架构

| Crate | 职责 | 代码行数 | 测试数 |
|-------|------|---------|--------|
| deneb-core | 数据类型、绘图指令、比例尺、解析器、降采样 | 5934 | 134 + 5 doc |
| deneb-component | 图表类型（Line/Bar/Scatter/Area）、布局、主题 | 5122 | 76 |
| deneb-wit | WASI 集成层、WIT 类型转换、lib_mode | 1200 | 20 |
| deneb-wit-wasm | WASI Component Model 导出（wit-bindgen 0.57） | 400 | 4 集成 |
| deneb-demo | 桌面演示（tiny-skia + wasmtime host） | 1800 | — |

## 关键依赖

| 依赖 | 版本 | 用途 | Crate |
|------|------|------|-------|
| serde / serde_json | — | JSON 序列化 | core, wit |
| chrono | — | 时间戳处理 | core |
| arrow / parquet | 可选 | 二进制数据格式 | core, demo |
| tiny-skia | 0.12 | CPU 2D 渲染 | demo |
| fontdue | 0.9 | 文本栅格化 | demo |
| winit | 0.30 | 窗口管理 | demo |
| softbuffer | 0.4 | 像素缓冲 | demo |
| wasmtime | 44 | WASM 运行时 | demo |
| wasmtime-wasi | 44 | WASI 支持 | demo |
| wit-bindgen | 0.57 | WIT 绑定生成 | wit-wasm |

## 功能清单

### 数据格式
- CSV（内建解析器，类型推断）
- JSON（对象数组 + 列式格式）
- Arrow IPC（limpuai:data 组件，运行时动态链接）
- Parquet（limpuai:data 组件，运行时动态链接）

### 图表类型
- Line（折线图，支持多系列颜色分组）
- Bar（柱状图，band scale + 自动间距）
- Scatter（散点图，支持大小/颜色映射）
- Area（面积图，支持堆叠 + 多系列）

### WASI 集成
- WIT 接口：data-parser（csv/json/arrow/parquet）+ chart-renderer
- 外部组件：limpuai:data/arrow-parser + limpuai:data/parquet-parser（WIT import，运行时链接）
- 库调用模式（宿主嵌入）
- 独立组件模式（wasm32-wasip2 目标）
- 类型编码：DrawCmd 展平、Path 段前缀编码、Text anchor/baseline 数字映射
- 类型映射：Arrow 物理类型 → deneb 语义类型（arrow_type_to_semantic）
- 字段推断：WitChartSpec 字段名 → DataTable 列类型 → Field 编码

### 交互支持
- HitRegion + BoundingBox 命中检测
- 坐标反查（像素 → 数据索引）
- 交互元数据与指令同步生成

## 编译目标
- Native：x86_64 / aarch64（标准 cargo build）
- WASM：wasm32-wasip2（cargo build -p deneb-wit-wasm --target wasm32-wasip2）

## 文档
- docs/：Mint 框架（architecture, line-chart, bar-chart, scatter-chart, area-chart, webassembly, demo）
- .specs/wasm-arrow-parquet/：Arrow/Parquet 集成需求、设计、任务追踪
- CLAUDE.md：开发规范、常见误区、Demo 运行方式
