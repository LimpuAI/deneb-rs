# 架构设计

## 分层总览

deneb-rs 采用四层架构，每层职责明确、单向依赖：

```mermaid
flowchart TB
    subgraph 应用层
        demo[deneb-demo<br/>桌面演示]
        wasm[WASM Host<br/>wasmtime]
    end

    subgraph 集成层
        wit[deneb-wit<br/>WASI 集成]
        witwasm[deneb-wit-wasm<br/>Component 导出]
    end

    subgraph 图表层
        comp[deneb-component<br/>图表组装]
    end

    subgraph 核心层
        core[deneb-core<br/>纯计算]
    end

    demo --> comp
    demo --> wit
    wasm --> witwasm
    witwasm --> wit
    wit --> comp
    wit --> core
    comp --> core
```

## Crate 职责

| Crate | 职责 | 关键导出 |
|-------|------|---------|
| **deneb-core** | 数据类型、绘图指令、比例尺、解析器 | `DataTable`, `DrawCmd`, `Scale`, `RenderLayers` |
| **deneb-component** | 图表类型实现、布局、主题 | `LineChart`, `BarChart`, `ChartSpec`, `Theme` |
| **deneb-wit** | WASI 集成层，类型转换 | `WitDrawCmd`, `WitRenderResult`, `lib_mode` |
| **deneb-wit-wasm** | WASI Component Model 导出 | WIT 接口实现 |
| **deneb-demo** | 桌面演示 + WASM Host | `TinySkiaRenderer`, `WasmHost`, `DemoApp` |

## 渲染管线

从原始数据到像素的完整数据流：

```mermaid
flowchart LR
    subgraph 数据解析
        raw[原始字节<br/>CSV/JSON/Arrow/Parquet] --> parser[Parser] --> dt[DataTable]
    end

    subgraph 图表组装
        dt --> spec[ChartSpec<br/>+ Theme]
        spec --> layout[布局计算<br/>+ 比例尺映射]
        layout --> render[图表渲染]
        render --> output[ChartOutput<br/>DrawCmd 分层 + HitRegion]
    end

    subgraph 像素渲染
        output --> skia[TinySkiaRenderer<br/>DrawCmd → Pixmap]
        skia --> display[窗口显示<br/>winit + softbuffer]
    end
```

## 核心数据类型

### 数据层

```mermaid
classDiagram
    class DataTable {
        +Vec~Column~ columns
        +Schema schema
        +row_count() usize
        +column_count() usize
    }

    class Schema {
        +Vec~(String, DataType)~ fields
        +index_of(name) Option~usize~
        +type_of(name) Option~DataType~
    }

    class Column {
        +String name
        +DataType data_type
        +Vec~FieldValue~ values
    }

    class FieldValue {
        <<enum>>
        Numeric(f64)
        Text(String)
        Timestamp(f64)
        Bool(bool)
        Null
    }

    class DataType {
        <<enum>>
        Quantitative
        Temporal
        Nominal
        Ordinal
    }

    DataTable "1" *-- "1" Schema
    DataTable "1" *-- "many" Column
    Column *-- FieldValue
    Column --> DataType
```

### 绘图指令

```mermaid
classDiagram
    class DrawCmd {
        <<enum>>
        Rect
        Circle
        Path
        Text
        Group
    }

    class PathSegment {
        <<enum>>
        MoveTo(x, y)
        LineTo(x, y)
        BezierTo(cp1, cp2, end)
        QuadraticTo(cp, end)
        Arc(cx, cy, r, start, end, ccw)
        Close
    }

    class FillStyle {
        <<enum>>
        Color(String)
        Gradient(Gradient)
        None
    }

    class StrokeStyle {
        <<enum>>
        Color(String)
        None
    }

    class TextStyle {
        +String font_family
        +f64 font_size
        +FontWeight font_weight
        +FontStyle font_style
        +FillStyle fill
    }

    DrawCmd --> PathSegment : Path variant
    DrawCmd --> FillStyle : fill
    DrawCmd --> StrokeStyle : stroke
    DrawCmd --> TextStyle : Text variant
```

### 分层渲染

```mermaid
flowchart TB
    subgraph RenderLayers
        direction TB
        L0[Background<br/>z-index: 0]
        L1[Grid<br/>z-index: 1]
        L2[Axis<br/>z-index: 2]
        L3[Data<br/>z-index: 3]
        L4[Legend<br/>z-index: 4]
        L5[Title<br/>z-index: 5]
        L6[Annotation<br/>z-index: 6]
    end

    subgraph ChartOutput
        layers[RenderLayers]
        hit_regions[Vec~HitRegion~]
    end

    layers --> RenderLayers
```

每层独立渲染，支持脏标记（dirty flag）的增量更新。

## 图表组装流程

以折线图为例，展示 `ChartSpec + DataTable → ChartOutput` 的完整过程：

```mermaid
sequenceDiagram
    participant App as 应用
    participant Chart as LineChart
    participant Layout as Layout
    participant Scale as Scale
    participant Core as deneb-core

    App->>Chart: render(spec, theme, table)
    Chart->>Layout: compute_layout(spec, theme)
    Layout-->>Chart: LayoutResult { plot_area, axes }

    Chart->>Scale: create_x_scale(data, plot_area)
    Scale-->>Chart: LinearScale

    Chart->>Scale: create_y_scale(data, plot_area)
    Scale-->>Chart: LinearScale

    loop 每个数据点
        Chart->>Scale: map(value)
        Scale-->>Chart: pixel_position
        Chart->>Core: DrawCmd::Path / Circle / Rect
    end

    Chart->>Core: RenderLayers::new()
    Chart->>Core: Layer::new(LayerKind::Background)
    Chart->>Core: Layer::new(LayerKind::Grid)
    Chart->>Core: Layer::new(LayerKind::Axis)
    Chart->>Core: Layer::new(LayerKind::Data)
    Chart->>Core: Layer::new(LayerKind::Title)

    Chart-->>App: ChartOutput { layers, hit_regions }
```

## WASI Component Model

### 双路径架构

deneb-rs 支持两种渲染路径：

```mermaid
flowchart TB
    subgraph Direct Path
        direction TB
        d1[ChartSpec + DataTable] --> d2[LineChart::render]
        d2 --> d3[ChartOutput]
        d3 --> d4[DrawCmd]
    end

    subgraph WASM Path
        direction TB
        w1[ChartSpec + data bytes] --> w2[wasmtime Host]
        w2 --> w3[WIT Component<br/>deneb-wit-wasm]
        w3 --> w4[WitRenderResult]
        w4 --> w5[WitDrawCmd]
    end

    d4 --> renderer[TinySkiaRenderer]
    w5 --> renderer
    renderer --> pixels[Pixmap → Display]
```

### WIT 接口

```mermaid
flowchart LR
    subgraph Host
        wasmtime[wasmtime Engine]
        bindgen[host bindgen!]
    end

    subgraph Component["deneb-viz Component"]
        guest[wit-bindgen generate!]
        impl[DenebVizComponent]
    end

    subgraph Parsers["limpuai:data Components"]
        arrow[arrow-parser]
        parquet[parquet-parser]
    end

    subgraph deneb-wit
        lib_mode[lib_mode API]
        convert[类型转换]
    end

    wasmtime -->|WIT ABI| guest
    bindgen -->|类型安全调用| wasmtime
    guest --> impl
    impl -->|arrow/parquet 委托| arrow
    impl -->|arrow/parquet 委托| parquet
    impl -->|csv/json| lib_mode
    lib_mode --> convert
    convert --> deneb-core
```

**导出接口：**

| 接口 | 函数 | 说明 |
|------|------|------|
| `data-parser` | `parse-csv` | 解析 CSV 数据 |
| `data-parser` | `parse-json` | 解析 JSON 数据 |
| `data-parser` | `parse-arrow` | 解析 Arrow IPC（委托 limpuai:data/arrow-parser） |
| `data-parser` | `parse-parquet` | 解析 Parquet（委托 limpuai:data/parquet-parser） |
| `chart-renderer` | `render` | 渲染图表 |
| `chart-renderer` | `hit-test` | 命中测试 |

### 类型转换策略

WIT 不支持递归类型和 Rust 复杂枚举，需要展平转换：

| 内部类型 | WIT 类型 | 转换策略 |
|---------|---------|---------|
| `DrawCmd`（枚举） | `WitDrawCmd`（扁平 record） | `cmd_type` 字符串 + `params` 数组 |
| `DrawCmd::Group`（递归） | `group_depth: u32` | 递归展平为线性列表 |
| `PathSegment`（枚举） | `params: list<f64>` | 类型编码前缀：0=MoveTo, 1=LineTo, ... |
| `TextAnchor` / `TextBaseline` | `params` 中的数字编码 | 0/1/2/3 映射 |
| `DataTable`（列式） | `WitDataTable`（行式） | 行列转置 |
| `FillStyle`（枚举） | `fill: option<string>` | 仅保留 Color 变体 |

## 依赖关系

```mermaid
flowchart LR
    core[deneb-core<br/>serde · chrono] --> comp[deneb-component]
    core --> wit[deneb-wit<br/>base64]
    comp --> wit
    wit --> witwasm[deneb-wit-wasm<br/>wit-bindgen 0.57]
    limpuai["limpuai:data<br/>wit/deps/limpuai-data/"] -.->|WIT import| witwasm

    comp --> demo[deneb-demo<br/>tiny-skia · fontdue<br/>winit · wasmtime]
    wit --> demo
```

| 外部依赖 | 用途 | 所在 Crate |
|---------|------|-----------|
| `serde` / `serde_json` | 序列化 | core, wit |
| `chrono` | 时间戳处理 | core |
| `arrow` / `parquet` | 二进制数据格式（可选） | core |
| `tiny-skia` | CPU 2D 渲染 | demo |
| `fontdue` | 文本栅格化 | demo |
| `winit` + `softbuffer` | 窗口管理 | demo |
| `wasmtime` | WASM 运行时 | demo |
| `wit-bindgen` 0.57 | WIT 绑定生成 | wit-wasm |

## 设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| 渲染输出 | Canvas 2D 指令序列 | 后端无关，可对接任意渲染器 |
| 数据模型 | 列式存储（内部）+ 行式（WIT） | 列式利于分析计算，行式利于跨语言传输 |
| WASM 模式 | WASI Component Model | 标准化组件模型，类型安全 |
| WIT 类型 | 展平 record | WIT 不支持递归类型 |
| 主题系统 | Trait 抽象 | 可扩展（DefaultTheme、DarkTheme） |
| 图层系统 | 7 层固定 + 自定义 | 脏标记支持增量渲染 |
