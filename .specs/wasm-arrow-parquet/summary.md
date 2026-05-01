# wasm-arrow-parquet Feature Summary

## 概述
deneb-rs 通过 WASI Component Model 运行时动态链接 limpuai-wit 解析器组件，实现 Arrow IPC 和 Parquet 格式数据支持。

## 实现方案

deneb-viz 通过 WIT `import` 声明依赖 `limpuai:data/arrow-parser` 和 `limpuai:data/parquet-parser`。宿主在实例化时通过 wasmtime Linker 注册解析器组件实例。deneb-viz 内部的 `parse-arrow` / `parse-parquet` 和 `render(format="arrow"/"parquet")` 自动委托给这些导入。

## 关键指标

| 指标 | 值 |
|------|-----|
| WASM 组件体积 | ~519KB |
| Workspace 测试 | 247 passed |
| Doc 测试 | 5 passed |
| WASM 集成测试 | 4 passed（parse_arrow, parse_parquet, render_arrow, render_parquet） |
| 修改文件数 | 15 |
| 新增代码行 | ~640 |
| 删除代码行 | ~210 |

## 技术决策

1. **WIT import 委托 vs 宿主组合**：选择 deneb-viz import parser，宿主只管实例化。好处是上层负担轻，deneb-viz 对外接口不变。
2. **copy-to-deps WIT 管理**：limpuai-wit 的 WIT 文件复制到 `deneb-wit/wit/deps/limpuai-data/`，移除版本后缀，版本通过 crate 锁定。
3. **嵌套实例导出导航**：wasmtime 44 没有 `get_instance`，通过 `get_export_index` 两步定位（先找嵌套实例索引，再找函数索引）。
4. **Arrow 类型映射**：`arrow_type_to_semantic()` 将 Int64/Float64 → quantitative，Utf8 → nominal，Date32 → temporal。
5. **字段类型推断**：`wit_chart_spec_with_table()` 从 DataTable 列 DataType 推断 Field 编码，修复 bar chart category 标签缺失。

## Demo 变更

| Demo | 数据格式 | WASM 需要 --deps |
|------|---------|-----------------|
| demo-line | CSV | 否 |
| demo-bar | CSV | 否 |
| demo-scatter | Parquet | 是 |
| demo-area | Parquet | 是 |

## 新增文件
- `crates/deneb-wit/wit/deps/limpuai-data/` — WIT 依赖
- `crates/deneb-demo/tests/wasm_arrow_parquet.rs` — 集成测试
- `CLAUDE.md` — 开发规范
