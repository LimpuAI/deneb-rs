# wasm-arrow-parquet Tasks

## Progress
Goal: deneb-rs 集成 limpuai-wit 解析器，支持 Arrow/Parquet 格式
Status: 10/10 (100%)
Current: Done
Next: cc-end

## Tasks
- [x] 1. WIT 跨包依赖 — import limpuai:data 解析器，copy WIT 到 deps/ - ref: design Changes 1-2
- [x] 2. wit-bindgen 0.57 升级 + generate_all — 支持跨包类型生成 - ref: design Changes 3
- [x] 3. deneb-wit-wasm 实现委托 — parse-arrow/parquet 委托给 limpuai:data - ref: design Changes 4
- [x] 4. arrow_type_to_semantic 类型映射 — Arrow 物理类型 → deneb 语义类型 - ref: design Changes 4
- [x] 5. wit_chart_spec_with_table 字段类型推断 — 从 DataTable 推断 Field 编码 - ref: design Changes 5
- [x] 6. WasmHost 运行时动态链接 — ParserPaths::from_dir + 嵌套实例导航 - ref: design Changes 6
- [x] 7. Demo 重组 — scatter/area 改用 Parquet，统一 CLI 参数解析 - ref: design Changes 7-9
- [x] 8. 集成测试 — 4 个 WASM Arrow/Parquet 测试 - ref: design Changes 10
- [x] 9. 全量测试验证 — 247 tests + 5 doctests + 4 WASM 集成测试通过
- [x] 10. 文档更新 — webassembly.md, demo.md, CLAUDE.md, architecture.md

## Notes
- wasmtime 44 没有 Instance::get_instance，需要 get_export_index 两步定位嵌套实例导出
- WIT `use` 不支持版本后缀，只能在 package 声明中加版本
- wit-bindgen `generate_all` 是裸标志语法（`generate_all,` 而非 `generate_all: true,`）
- limpuai-wit .wasm 修改 WIT 后必须重编译，否则导出名不匹配
