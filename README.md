# deneb-rs

Backend-agnostic Rust visualization library that outputs Canvas 2D instruction sequences. Designed for WASI Component Model.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## Features

- **Backend-agnostic** — Outputs Canvas 2D instruction sequences, decoupled from any rendering engine
- **Grammar of Graphics** — Declarative API inspired by Vega-Lite, type-safe builder pattern
- **Data-visual separation** — Data parsing, encoding, and instruction generation are fully decoupled
- **Layer-based rendering** — 7 layers with dirty flags, host controls incremental repaint
- **WASM-first** — Native WASI Component Model support (wasm32-wasip2 target)
- **Multi-format data** — CSV, JSON, Arrow IPC, Parquet

## Chart Types

| Chart | Mark | Description |
|-------|------|-------------|
| Line | `Mark::Line` | Time series, continuous trends |
| Bar | `Mark::Bar` | Categorical comparison |
| Scatter | `Mark::Scatter` | Distribution, correlation |
| Area | `Mark::Area` | Trend with magnitude |
| Pie | `Mark::Pie` | Proportional comparison, donut chart |
| Histogram | `Mark::Histogram` | Distribution, Sturges auto-binning |
| BoxPlot | `Mark::BoxPlot` | Statistical summary, IQR outlier detection |
| Waterfall | `Mark::Waterfall` | Cumulative increment/decrement |
| Candlestick | `Mark::Candlestick` | OHLC financial chart |
| Radar | `Mark::Radar` | Multi-dimensional polar comparison |
| Heatmap | `Mark::Heatmap` | Color-mapped matrix |
| Strip | `Mark::Strip` | Beeswarm distribution by category |
| Sankey | `Mark::Sankey` | Flow diagram with Bézier ribbons |
| Chord | `Mark::Chord` | Circular relationship diagram |
| Contour | `Mark::Contour` | Isoline contour map (marching squares) |

## Quick Start

**Native path:**

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
// output.layers — layered drawing instructions
// output.hit_regions — interaction hit regions
```

**WASM Component:**

```bash
# Build WASI Component (~498KB release)
cargo build -p deneb-wit-wasm --target wasm32-wasip2 --release

# Run demo with WASM path
cargo run --bin demo-line -- --wasm target/wasm32-wasip2/release/deneb_wit_wasm.wasm

# Parquet/Arrow demos need --deps for parser components
cargo run --bin demo-scatter -- \
  --wasm target/wasm32-wasip2/release/deneb_wit_wasm.wasm \
  --deps ../limpuai-wit/target/wasm32-wasip2/release
```

## Crate Structure

| Crate | Description |
|-------|-------------|
| [deneb-core](crates/deneb-core) | Data types, drawing instructions, scales, parsers, downsampling, interaction |
| [deneb-component](crates/deneb-component) | Chart types (15 types), layout engine, theme system, shared rendering helpers |
| [deneb-wit](crates/deneb-wit) | WASI integration layer, WIT type conversion, lib_mode API |
| [deneb-wit-wasm](crates/deneb-wit-wasm) | WASI Component Model export (wit-bindgen 0.57) |
| [deneb-demo](crates/deneb-demo) | Desktop demo (tiny-skia + wasmtime host + winit window) |

## Demo

15 demo binaries, each with native and WASM rendering paths:

```bash
# Native rendering
cargo run --bin demo-line
cargo run --bin demo-bar
cargo run --bin demo-scatter
cargo run --bin demo-area
cargo run --bin demo-pie
cargo run --bin demo-histogram
cargo run --bin demo-boxplot
cargo run --bin demo-waterfall
cargo run --bin demo-candlestick
cargo run --bin demo-radar
cargo run --bin demo-heatmap
cargo run --bin demo-strip
cargo run --bin demo-sankey
cargo run --bin demo-chord
cargo run --bin demo-contour

# WASM rendering
cargo run --bin demo-line -- --wasm target/wasm32-wasip2/release/deneb_wit_wasm.wasm
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  Host Application                                   │
│    ┌──────────────┐         ┌──────────────────┐    │
│    │ Direct Path  │         │   WASM Path      │    │
│    │ Chart::render│         │ wasmtime Host    │    │
│    └──────┬───────┘         └────────┬─────────┘    │
│           │                          │               │
│           ▼                          ▼               │
│    DrawCmd (native)    WitDrawCmd (WASM boundary)    │
│           │                          │               │
│           └──────────┬───────────────┘               │
│                      ▼                               │
│              TinySkiaRenderer                        │
│              (Canvas 2D → pixels)                    │
└─────────────────────────────────────────────────────┘
```

## Build & Test

```bash
# Build
cargo build --workspace

# Test (373 tests, excluding slow WASM integration tests)
cargo test --workspace --exclude deneb-demo

# Lint
cargo clippy --workspace
```

## License

Licensed under the [MIT License](LICENSE).

Copyright (c) 2026 StarEcho Pte. Ltd.

## 打包（aixpack 标准插件包）

宿主应用装载的标准插件包由 [aixpack](https://github.com/LimpuAI/aixpack)（LimpuAI 生态工具链，`cargo install --git https://github.com/LimpuAI/aixpack`）从本仓 `pack.toml` 产出：

```bash
aixpack --root <本仓> build       # wasip2 release 组件 → dist/staging/
aixpack --root <本仓> pkg --zip   # dist/<author>--<name>/ + 标准包 zip（唯一顶层）
aixpack --root <本仓> verify --wit-root <WIT路径>   --contracts-root <契约仓> --registry-root <登记处仓>
```

verify 三查：WIT 双侧一致（本仓 wit 副本 ↔ 契约权威副本）/ world ⊆ 扩展点登记处 / 包内哈希三方一致。

快捷脚本：`scripts/verify-package.sh`（三查；兄弟仓同级布局下零参直跑——`CONTRACTS_ROOT`（兼容 `ECHODAWN_ROOT`）/ `AIXPACK_ROOT` / `REGISTRY_ROOT` 环境覆写仍可用）、`scripts/install-hooks.sh`（pre-commit 安装器）。（注：本仓曾长期位于 G:/code 导致兄弟探测失效，2026-09-10 归位 ../deneb-rs 后默认路径全通）

工具链安装：`cargo install --git https://github.com/LimpuAI/aixpack`（安装后 `scripts/verify-package.sh` 自动优先 PATH；未安装时兜底兄弟 checkout 构建产物）。
