# More Chart Types Requirements

## What we need
在 deneb-rs 中实现 11 种新图表类型，参考 lodviz-rs 的图表语义和算法实现，适配 deneb-rs 的 Canvas 2D 指令架构。新增图表：Pie、Histogram、BoxPlot、Waterfall、Candlestick、Radar、Heatmap、Strip、Sankey、Chord、Contour。

## Input & Output
**Input**: ChartSpec（声明式配置，扩展 Encoding 通道）+ DataTable（列式数据）
**Output**: ChartOutput（分层 Canvas 2D 指令 + HitRegion 交互元数据）

## 架构扩展点
- **Mark 枚举**：新增 11 个变体（Pie, Histogram, BoxPlot, Waterfall, Candlestick, Radar, Heatmap, Strip, Sankey, Chord, Contour）
- **Encoding 通道**：扩展 open/high/low/close/theta/color2 通道
- **DrawCmd**：新增 Arc 变体（弧形扇形，Pie/Radar 需要）
- **deneb-core 算法**：移植 Sankey 布局、Chord 布局、Contour（marching squares）、KDE（核密度估计）、beeswarm 算法
- **deneb-component chart 模块**：每个图表类型一个独立文件

## Success criteria
- [ ] 11 种图表均可通过 ChartSpec builder 创建并渲染
- [ ] 每种图表有空数据、单数据点、极端值的正确降级处理
- [ ] 每种图表生成正确的 HitRegion 交互元数据
- [ ] 所有图表的 Y 轴 include_zero 规则遵循 CLAUDE.md 规范（Bar/Waterfall/Histogram 必须，其余不必须）
- [ ] WIT 接口（deneb-wit-wasm）支持新增的 Mark 类型
- [ ] 每种图表至少有一个 demo binary（deneb-demo）
- [ ] `cargo test --workspace --exclude deneb-demo` 全部通过
- [ ] `cargo clippy --workspace` 无新增 warning

## Edge cases
- **Pie 空数据**: 返回空指令，不 panic
- **K线 OHLC 不完整**: 缺少 open/high/low/close 任一字段返回 InvalidConfig 错误
- **Heatmap 颜色映射**: color 通道编码数值，需 LinearScale 映射到颜色梯度
- **Sankey 零流量**: 零宽度 ribbon 不绘制但保留节点
- **Contour 少于 3 个点**: 降级为散点图提示
- **Histogram 空 bin**: 所有值相同 → 单 bin
- **Radar 单维度**: 退化为从中心向外的射线
- **BoxPlot 少于 5 个数据点**: 降级为简单范围展示
- **StripChart 单类别**: 单列蜂群布局
- **Chord 空矩阵**: 返回空指令
- **Waterfall 全正值/全负值**: 基线在底部/顶部

## Might-it-be 关联
- `.specs/might-it-be.md` "Forward-Looking: 更多图表类型" 提到 Chart trait 已就绪、Layout 引擎可复用、Encoding 可能需要扩展（shape 通道）——本次实现验证了这些扩展点
- "Canvas 2D API 映射层完整指令集" 中 Gradient 渐变填充可用于 Heatmap 颜色映射
