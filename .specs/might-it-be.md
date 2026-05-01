# Might-It-Be

开发过程中发现的前瞻性想法、推迟的工作和设计争议。

## Future TODO: 增量更新 update-data

WIT 接口中 `update-data` 函数在 design.md 中定义但未实现。当前每次渲染都是完整重算。增量更新需要：
- diff 机制检测数据变化范围
- 只重标记受影响的 Layer（dirty flag 已就位）
- 影响范围：deneb-wit WIT 接口、deneb-component 增量渲染接口

推迟原因：当前全量渲染在 10K 数据点内性能足够，增量更新是优化而非功能缺失。

## Future TODO: Canvas 2D API 映射层完整指令集

当前 CanvasOp 枚举定义了完整指令集（transform、clip、composite 等），但实际仅使用了 DrawCmd 子集（Rect、Path、Circle、Text、Group）。完整指令集覆盖需要：
- Transform 矩阵变换
- Clip 裁剪区域
- Composite 混合模式
- Gradient 渐变填充（FillStyle::Gradient 已定义但未在图表中使用）

推迟原因：4 种图表类型不需要这些高级特性。

## Future TODO: 降采样自动触发阈值

LTTB 和 M4 算法已实现，但自动触发阈值未确定。当前需要用户手动调用。需要：
- 可配置的默认阈值（如 10K 点）
- 自动检测数据量并提示
- 降采样元数据输出供宿主提示用户

推迟原因：需求文档中列为 Open decision。

## Future TODO: JSON Schema 约定

JSON 数据输入的具体 schema 约定未确定。当前支持：
- 对象数组 `[{x: 1, y: 2}, ...]`
- 列式格式 `{x: [1, 2], y: [2, 3]}`
- 嵌套对象自动展平

推迟原因：需求文档中列为 Open decision。

## Forward-Looking: 更多图表类型

当前 4 种图表类型架构验证完毕。新增图表类型（Pie、Heatmap、Histogram、BoxPlot）的扩展点：
- Chart trait 已定义（render + validate_data）
- Layout 引擎可复用
- Theme trait 天然支持新图表
- Encoding 通道可能需要扩展（如 shape 通道用于 Scatter 形状映射）

## Forward-Looking: 响应式集成方案

响应式设计原则已内置（分层 + dirty flag），但宿主集成方案待设计：
- 数据变化 → 只重标记受影响层
- 窗口尺寸变化 → 全量重算布局
- 主题切换 → 重标记所有样式相关层
- 需要宿主提供调度器和 DOM/Canvas 绑定

## Controversy: WIT 递归类型限制

WIT 不支持递归类型，导致 DrawCmd::Group 的 children 无法直接映射。当前方案是展平为线性列表 + group_depth 标记。备选方案：
- 保持嵌套但用 JSON 字符串编码（反序列化成本高）
- 多次 WIT 调用按层获取（网络开销）

当前方案是最小侵入性的，但牺牲了 Group 的结构信息。如果未来 Group 内需要独立 transform，需要重新评估。

## Controversy: DataTable 列式 vs 行式

内部列式存储利于分析计算（Scale 计算、降采样），但 WIT 边界需要行式传输（行列转置）。性能权衡：
- 列式 → 行式转换在大数据集上有拷贝开销
- 可考虑零拷贝方案（WASM 共享内存）

当前方案对 10K-100K 数据点足够。

## Future TODO: 解析器组件自动下载

当前 `--deps <dir>` 要求用户手动指定 limpuai-wit 编译输出目录。可考虑：
- 从 registry 或 URL 自动下载解析器组件
- 版本锁定文件（类似 lock file）
- 编译脚本自动拉取依赖

推迟原因：当前手动指定已满足需求。

## Forward-Looking: WASM 组件测试性能

4 个 WASM 集成测试耗时 ~130 秒（每个 ~30 秒），主要瓶颈是 wasmtime 引擎初始化和组件实例化。可考虑：
- 测试间共享 Engine 实例
- 使用 `InstancePre` 预实例化
- 并行化测试执行

## Controversy: WIT import 委托 vs 宿主组合

deneb-viz 通过 WIT import 导入 limpuai:data 解析器，宿主在 Linker 中注册。备选方案是宿主根据 format 选择调不同组件。当前选择 import 委托的优势：
- 宿主代码简单（一个组件实例）
- deneb-viz 对外接口统一
- format 路由在组件内部处理

劣势：
- deneb-viz 必须在编译时知道所有 parser 接口
- 新增格式需要修改 deneb-viz WIT 并重编译
