# deneb-rs

deneb-rs 是一个后端无关的 Rust 可视化库，输出 Canvas 2D 指令序列。

## 核心特性

- **后端无关** — 输出 Canvas 2D 指令序列，不绑定具体渲染引擎
- **数据与绘图逻辑分离** — 数据解析、图表组装、渲染输出三层解耦
- **绘图语法** — API 启发自 Vega-Lite，声明式图表规格
- **细粒度响应式** — 数据和接口对响应式友好，响应式逻辑由宿主承接
- **为 WebAssembly 而生** — WASI Component Model 原生支持
- **多格式数据源** — 支持 CSV、JSON、Arrow IPC、Parquet

## 支持的图表类型

| 图表 | Mark | 说明 |
|------|------|------|
| 折线图 | `Line` | 时间序列、连续数据趋势 |
| 柱状图 | `Bar` | 分类数据对比 |
| 散点图 | `Scatter` | 数据分布、相关性 |
| 面积图 | `Area` | 趋势与量级展示 |

## 快速开始

**直接调用（Native）：**

```rust
use deneb_component::{ChartSpec, Encoding, Field, LineChart, Mark, DefaultTheme};
use deneb_core::parser::csv::parse_csv;

let table = parse_csv("x,y\n1,10\n2,20\n3,15")?;
let spec = ChartSpec::builder()
    .mark(Mark::Line)
    .encoding(Encoding::new()
        .x(Field::quantitative("x"))
        .y(Field::quantitative("y")))
    .width(800.0)
    .height(600.0)
    .build()?;

let output = LineChart::render(&spec, &DefaultTheme, &table)?;
// output.layers — 分层绘图指令
// output.hit_regions — 交互命中区域
```

**WASM Component：**

```bash
# 编译 WASI Component
cargo build -p deneb-wit-wasm --target wasm32-wasip2 --release

# 在宿主中加载
cargo run --bin demo-line -- --wasm target/wasm32-wasip2/release/deneb_wit_wasm.wasm
```
