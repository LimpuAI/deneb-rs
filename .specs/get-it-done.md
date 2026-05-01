# Get-It-Done

开发过程中已解决的问题和决策记录。

## Resolved: DataTable Schema 缺失

**原始问题**（cc-review P1）：DataTable 仅有 `columns: Vec<Column>` 字段，缺少 `Schema` 元信息。`get_column` 通过线性遍历查找（O(n)）。

**解决方案**：DataTable 新增 `schema: Schema` 字段，`with_columns` 和 `add_column` 自动维护 schema 索引，`get_column`/`get_column_mut` 通过 schema 的 HashMap 查找（O(1)）。

**解决日期**：2026-04-30

## Resolved: BandScale 死代码

**原始问题**（cc-review P2）：`BandScale::index_of()` 标记 `#[allow(dead_code)]`，实际未被调用。`OrdinalScale` 有同名方法且在使用。

**解决方案**：直接移除 `BandScale::index_of()`。OrdinalScale 版本保留。

**解决日期**：2026-04-30

## Resolved: WIT 文本定位数据丢失

**原始问题**：WASM 版本 Y 轴标签渲染在轴上而非轴左侧，Native 版本正常。原因是 Text → WitDrawCmd 转换时 `font_size`、`anchor`、`baseline` 信息被丢弃。

**解决方案**：将 Text 参数编码到 params 数组 `[x, y, font_size, anchor_code, baseline_code]`，anchor 用 0/1/2 编码，baseline 用 0/1/2/3 编码。渲染端解码还原。

**解决日期**：2026-04-30

## Resolved: WIT 递归类型不可表达

**原始问题**：WIT 规范不支持递归类型，`DrawCmd::Group { children: Vec<DrawCmd> }` 无法直接映射到 WIT record。

**解决方案**：用 `group_depth: u32` 替代 children，递归展平为线性列表。`draw_cmd_to_wit_draw_cmd_flat(cmd, depth)` 遍历树结构，Group 的 children 递归调用并递增 depth。

**解决日期**：2026-04-29

## Resolved: wasmtime 44 API 变更

**原始问题**：wasmtime 44 中 `wasmtime_wasi::preview2` 模块不存在，`WasiCtx` 构建方式变更。

**解决方案**：使用 `wasmtime_wasi::WasiCtx::builder()` + `wasmtime_wasi::WasiView` trait + `wasmtime_wasi::p2::add_to_linker_sync()`。`bindgen!` 宏生成 `DenebViz` 结构，方法名格式为 `deneb_viz_data_parser()` / `deneb_viz_chart_renderer()`。

**解决日期**：2026-04-30

## Resolved: wit-bindgen 0.51 宏 API

**原始问题**：`export_world!` 宏在 wit-bindgen 0.51 中不存在，Guest trait 命名空间不确定。

**解决方案**：正确宏为 `export!(ComponentStruct)`。Guest traits 在 `exports::deneb::viz::data_parser::Guest` 和 `exports::deneb::viz::chart_renderer::Guest` 命名空间下。

**解决日期**：2026-04-29

## Resolved: wasmtime 44 嵌套实例导出访问

**原始问题**：wasmtime 44 的 `Instance` 没有 `get_instance` 方法。解析器 .wasm 组件的 `parse` 函数在嵌套实例导出内，无法通过 `instance.get_func(store, "parse")` 直接获取。

**解决方案**：使用 `get_export_index` 两步导航：先获取嵌套实例索引，再获取函数索引，最后 `get_func`。

**解决日期**：2026-05-01

## Resolved: Bar chart category 标签在 WASM 模式下缺失

**原始问题**：WASM 路径下 bar chart 的 x 轴 category 标签不显示。根因是 `wit_chart_spec_to_chart_spec` 将所有字段硬编码为 `Field::quantitative()`。

**解决方案**：新增 `wit_chart_spec_with_table()` 从 DataTable 列 DataType 推断 Field 编码。

**解决日期**：2026-05-01

## Resolved: Arrow 物理类型与 deneb 语义类型不匹配

**原始问题**：limpuai:data 解析器返回 Arrow 物理类型名（`Int64`），deneb 期望语义类型（`quantitative`），导致 render 失败。

**解决方案**：添加 `arrow_type_to_semantic()` 映射函数，在类型转换时执行映射。

**解决日期**：2026-05-01
