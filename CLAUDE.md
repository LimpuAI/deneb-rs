# deneb-rs 开发指南

项目概述和架构参见 [.specs/project-info.md](.specs/project-info.md)。

## 编译

```bash
# Native
cargo build --workspace

# WASM 组件（release）
cargo build -p deneb-wit-wasm --target wasm32-wasip2 --release

# 运行测试（排除慢速 WASM 集成测试）
cargo test --workspace --exclude deneb-demo
```

## 核心规范

### 字段类型推断

WIT 接口（`WitChartSpec`）只传字段名，不传类型。`wit_chart_spec_with_table` 从 `DataTable` 的列类型推断 `Field` 编码：
- `Nominal` / `Ordinal` → `Field::nominal()`
- `Temporal` → `Field::temporal()`
- `Quantitative` → `Field::quantitative()`

**禁止硬编码所有字段为 `quantitative`**。Bar chart 的 x 轴必须是 nominal，否则不会生成 category 标签。

### Arrow 物理类型映射

Arrow/Parquet 解析器返回的 `data_type` 是物理类型名（`Int64`, `Float64`, `Utf8`），必须通过 `arrow_type_to_semantic()` 映射为 deneb 语义类型（`quantitative`, `nominal`, `temporal`）。映射在 `deneb-wit-wasm/src/lib.rs` 的 `limpuai_dt_to_bindgen` 和 `limpuai_dt_to_wit` 中执行。

### WIT 版本管理

- WIT 文件中 **不加版本后缀**（`package limpuai:data;` 而非 `package limpuai:data@0.1.0;`）
- 版本通过 crate 版本管理
- 依赖的 WIT 文件 copy 到 `wit/deps/` 目录

### WASM 组件嵌套实例

wasmtime 44 的 `Instance` 没有 `get_instance` 方法。访问嵌套实例导出的函数需要两步 `get_export_index`：

```rust
let instance_idx = root.get_export_index(&mut store, None, "limpuai:data/arrow-parser")?;
let func_idx = root.get_export_index(&mut store, Some(&instance_idx), "parse")?;
let func = root.get_func(&mut store, &func_idx)?;
```

### Linker 注册解析器

`func_wrap` 的参数必须是元组语法 `(Vec<u8>,)` 而非 `Vec<u8>`，否则类型不满足 `ComponentNamedList`。

## Demo 运行

```bash
# CSV/JSON 格式（demo-line, demo-bar）
cargo run --bin demo-line
cargo run --bin demo-line -- --wasm target/wasm32-wasip2/release/deneb_wit_wasm.wasm

# Parquet 格式（demo-scatter, demo-area），WASM 模式需要 --deps
cargo run --bin demo-scatter
cargo run --bin demo-scatter -- \
  --wasm target/wasm32-wasip2/release/deneb_wit_wasm.wasm \
  --deps ../limpuai-wit/target/wasm32-wasip2/release
```

`--deps <dir>` 按文件名约定自动发现 `limpuai_wit_arrow.wasm` 和 `limpuai_wit_parquet.wasm`。

## 常见误区

1. **WIT `use` 不支持版本**：`use limpuai:data/types@0.1.0.field-value` 语法错误，版本只能在 `package` 声明中
2. **wit-bindgen `generate_all` 是裸标志**：写 `generate_all,` 而非 `generate_all: true,`
3. **WASM 必须用 wasm32-wasip2 编译**：x86_64 目标会产生 cabi 符号未定义错误
4. **修改 WIT 文件后必须重建 .wasm**：deneb-wit-wasm 和 limpuai-wit 的 .wasm 都需要重编译
5. **native 和 WASM 的 render 路径共享 deneb-core/deneb-component**：类型转换 round-trip 必须无损
