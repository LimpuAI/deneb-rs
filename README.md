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
```

## Crate Structure

| Crate | Description |
|-------|-------------|
| [deneb-core](crates/deneb-core) | Data types, drawing instructions, scales, parsers, downsampling, interaction |
| [deneb-component](crates/deneb-component) | Chart types (Line/Bar/Scatter/Area), layout engine, theme system |
| [deneb-wit](crates/deneb-wit) | WASI integration layer, WIT type conversion, lib_mode API |
| [deneb-wit-wasm](crates/deneb-wit-wasm) | WASI Component Model export (wit-bindgen 0.51) |
| [deneb-demo](crates/deneb-demo) | Desktop demo (tiny-skia + wasmtime host + winit window) |

## Demo

Four demo binaries, each with native and WASM rendering paths:

```bash
# Native rendering
cargo run --bin demo-line
cargo run --bin demo-bar
cargo run --bin demo-scatter
cargo run --bin demo-area

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

# Test (248 tests)
cargo test --workspace

# Lint
cargo clippy --workspace
```

## License

Licensed under the [MIT License](LICENSE).

Copyright (c) 2026 StarEcho Pte. Ltd.
