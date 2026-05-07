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

## Resolved: Bar chart 单系列所有柱子同色

**原始问题**：bar chart 在单系列（series_count == 1）时，所有 category 的柱子都使用 `palette[0]`，导致视觉上无法区分不同类别。同时 Cappuccino 主题的 10 色调色板全是棕/米色系，色相差极小。

**解决方案**：
1. bar.rs `render_bars` 将颜色选择移入内层循环，单系列按 `theme.series_color(bar_idx)` 分色，多系列按 `theme.series_color(series_idx)` 分色
2. Cappuccino 调色板从单色相棕色系替换为多色相暖色系（棕→橙→金→粉→绿→铜→赭→青→红→驼）

**解决日期**：2026-05-07

## Resolved: Bar chart Y 轴不从 0 开始

**原始问题**：`compute_axis_layout` 中的 Y 轴 domain 直接取数据 min-max，不包含 0。Bar chart 的柱子长度编码数值，截断轴会严重误导（经典 Fox News 数据可视化错误）。

**解决方案**：`compute_axis_layout` 新增 `include_zero` 参数。`compute_layout` 中 `spec.mark == Mark::Bar` 时为 Y 轴传 `true`，确保 domain 包含 0。其他图表类型传 `false`（位置编码不需要从 0 开始）。

**解决日期**：2026-05-07

## Resolved: 11 种新图表类型实现

**原始需求**：deneb-rs 仅支持 4 种图表（Line/Bar/Scatter/Area），需要扩展到 15 种以覆盖常见可视化场景。

**解决方案**：
1. Mark 枚举扩展 4→15 变体
2. Encoding 新增 7 个可选通道（open/high/low/close/theta/size/color2）
3. DrawCmd 新增 Arc 变体支持饼图/雷达图
4. 移植 5 个算法模块（kde, beeswarm, sankey_layout, chord_layout, contour）
5. 11 个独立图表渲染器文件
6. WIT 层完整支持所有新 Mark 类型
7. 15 个 demo binary + 11 个文档页
8. 共享渲染辅助提取到 chart/shared.rs

**解决日期**：2026-05-07

## Resolved: Y 轴 include_zero 扩展到 Histogram/Waterfall

**原始问题**（cc-review P1）：`compute_axis_layout` 的 `include_zero` 仅检查 `Mark::Bar`，Histogram/Waterfall 的 `layout.y_axis` 网格线不包含 0，导致网格与柱基线不对齐。

**解决方案**：`matches!(spec.mark, Mark::Bar | Mark::Histogram | Mark::Waterfall)`

**解决日期**：2026-05-07

## Resolved: 图表渲染代码重复

**原始问题**（cc-review P1）：15 个图表文件中 background/grid/axes/title 渲染逻辑高度重复。

**解决方案**：提取 `chart/shared.rs` 公共模块（6 个函数），所有直角坐标图表（9 个）和部分自定义图表（4 个）迁移使用。bar.rs 消除 214 行重复代码。

**解决日期**：2026-05-07

## Resolved: Encoding color2 字段缺失

**原始问题**（cc-review P2）：design.md 定义了 `color2: Option<Field>` 渐变终点色字段，但 Encoding 中未实现。

**解决方案**：添加 `color2` 字段 + builder 方法 + 测试。当前无图表使用，API 完整性补齐。

**解决日期**：2026-05-07

## Resolved: Contour demo 缺失 color 编码

**原始问题**（2nd review P0）：demo_contour.rs 的 Encoding 只有 x 和 y，缺少 color 通道。contour_chart.rs 回退到 y 列当值，导致等高线编码 y 坐标而非 value。

**解决方案**：Native 路径添加 `.color(Field::quantitative("value"))`，WASM 路径添加 `color_field: Some("value".to_string())`。

**解决日期**：2026-05-07

## Resolved: Pie validate_data 不覆盖 fallback 通道

**原始问题**（2nd review P1）：`build_slices()` 使用 `color.or(x)` 和 `theta.or(y)` fallback，但 `validate_data()` 只验证 color/theta。当用户只设 x 不设 color 时，validate 通过但 build_slices 读取未验证字段。

**解决方案**：validate_data 增加对 x（当 color None 时）和 y（当 theta None 时）的验证。

**解决日期**：2026-05-07

## Resolved: Pie/Radar 自定义 render_title

**原始问题**（2nd review P1）：pie.rs 和 radar.rs 有自定义 render_title 方法，与其他 13 种图表使用 `shared::render_title` 不一致。

**解决方案**：删除自定义方法，调用 `super::shared::render_title(theme, title, &plot_area)`。

**解决日期**：2026-05-07

## Resolved: Chord 4 个未使用变量

**原始问题**（2nd review P2）：chord.rs 中 `src_end_x/y`、`dst_start_x/y` 是上一轮 ribbon 重构的残留，clippy 报警。

**解决方案**：删除 4 个未使用变量声明。

**解决日期**：2026-05-07

## Resolved: Pie SliceData.value 死字段

**原始问题**（2nd review P2）：SliceData 结构体的 value 字段 never read（dead code）。

**解决方案**：移除 value 字段及相关初始化。

**解决日期**：2026-05-07

## Resolved: Line/Scatter/Area 缺少 validate_data

**原始问题**（2nd review P2）：line.rs、scatter.rs、area.rs 无独立 validate_data 方法，与项目其他 12/15 图表模式不一致。

**解决方案**：添加 validate_data 方法，检查 x/y 编码存在性及字段在数据中存在性。空数据时跳过字段检查（与 bar.rs 一致）。

**解决日期**：2026-05-07

## Resolved: Sankey layout 文件注释过时

**原始问题**（2nd review P2）：sankey_layout.rs 文件注释仍写 "BFS from source nodes"，但算法已改为 Kahn 最长路径拓扑排序。

**解决方案**：更新注释为 "Kahn's topological sort (longest path)"。

**解决日期**：2026-05-07
