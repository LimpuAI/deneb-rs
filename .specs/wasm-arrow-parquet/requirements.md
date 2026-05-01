# wasm-arrow-parquet Requirements

## What we need
deneb-rs 通过 WASI Component Model 运行时动态链接 limpuai-wit 解析器组件，支持 Arrow IPC 和 Parquet 格式数据。

## Architecture
```
宿主
  ├── limpuai_wit_arrow.wasm    (Arrow IPC 解析, ~1.2MB)
  ├── limpuai_wit_parquet.wasm  (Parquet 解析, ~6.4MB)
  └── deneb_wit_wasm.wasm       (csv/json 解析 + 图表渲染 + arrow/parquet 委托, ~519KB)

deneb-viz 通过 WIT import 声明依赖 limpuai:data/arrow-parser 和 limpuai:data/parquet-parser，
宿主在实例化时通过 Linker 注册解析器组件（运行时动态链接）。
```

## Success criteria
- [x] deneb-viz WIT 导入 limpuai:data/arrow-parser 和 limpuai:data/parquet-parser
- [x] deneb-wit-wasm 不依赖 arrow/parquet crate，体积保持 ~519KB
- [x] 解析器组件通过 `--deps <dir>` 参数自动发现并链接
- [x] arrow/parquet 解析和渲染通过 WASM 路径正常工作
- [x] Arrow 物理类型正确映射为 deneb 语义类型（arrow_type_to_semantic）
- [x] 字段类型从 DataTable 列类型自动推断（wit_chart_spec_with_table）
- [x] demo-scatter 和 demo-area 使用 Parquet 格式数据
- [x] 所有测试通过（247 workspace tests + 5 doctests + 4 WASM 集成测试）

## Edge cases
- 未提供解析器时注册 stub，调用返回明确错误
- render() 中 Arrow 物理类型名（Int64, Float64, Utf8）正确映射为语义类型
- Bar chart 的 x 轴 category 标签通过 nominal 类型正确生成
