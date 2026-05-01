# wasm-arrow-parquet Design

## Key decisions
- **deneb-viz 通过 WIT import 委托**: deneb-viz 在 WIT 中声明 `import limpuai:data/arrow-parser` 和 `limpuai:data/parquet-parser`，运行时由宿主通过 Linker 动态链接
- **运行时动态链接**: 宿主加载 parser .wasm 组件，实例化后通过 `get_export_index` 两步定位嵌套实例中的 `parse` 函数，注册到 Linker
- **Arrow 物理类型 → 语义类型映射**: `arrow_type_to_semantic()` 在 deneb-wit-wasm 中将 Int64/Float64/Utf8 等映射为 quantitative/nominal/temporal
- **字段类型推断**: `wit_chart_spec_with_table()` 从 DataTable 列类型推断 Field 编码，避免硬编码 quantitative
- **WIT 版本管理**: 移除 `@0.1.0` 后缀，版本通过 crate 管理，依赖 WIT copy 到 `wit/deps/`

## Changes
1. **world.wit** — 新增 `import limpuai:data/arrow-parser` 和 `import limpuai:data/parquet-parser`，新增 `parse-arrow` 和 `parse-parquet` 函数
2. **wit/deps/limpuai-data/** — 复制 limpuai-wit WIT 文件（移除版本后缀）
3. **deneb-wit-wasm/Cargo.toml** — wit-bindgen 升级到 0.57，启用 `generate_all`
4. **deneb-wit-wasm/src/lib.rs** — 实现 cross-package import 委托，arrow_type_to_semantic 类型映射
5. **deneb-wit/src/lib.rs** — 新增 `wit_chart_spec_with_table` 字段类型推断，新增 `render_from_wit_table`
6. **deneb-demo/wasm_host.rs** — `ParserPaths::from_dir` 自动发现，`load_parser_func` 嵌套实例导航，`from_file_with_parsers` 构造器
7. **deneb-demo/lib.rs** — `parse_wasm_args()` 公共 CLI 参数解析，`WasmArgs` 结构体
8. **deneb-demo/sample_data.rs** — Parquet 测试数据生成（scatter_chart_parquet, area_chart_parquet）
9. **demo binary** — 统一使用 `parse_wasm_args()`，scatter/area 改用 Parquet 格式
10. **集成测试** — `tests/wasm_arrow_parquet.rs` 4 个测试覆盖 parse 和 render

## WIT dependency management
```
limpuai-wit/wit/          →  deneb-wit/wit/deps/limpuai-data/
  types.wit                    types.wit
  arrow-parser.wit             arrow-parser.wit
  parquet-parser.wit           parquet-parser.wit
```
copy-to-deps 模式，版本通过 crate 版本锁定。

## Host integration pattern
```rust
// 宿主实例化 deneb-viz，同时链接解析器组件
let mut host = WasmHost::from_file_with_parsers(
    "deneb_wit_wasm.wasm",
    ParserPaths::from_dir("../limpuai-wit/target/wasm32-wasip2/release"),
)?;

// arrow/parquet 数据直接传给 deneb-viz，内部自动委托
let result = host.render(&parquet_data, "parquet", &spec)?;
```
