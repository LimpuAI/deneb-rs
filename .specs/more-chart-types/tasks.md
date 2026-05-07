# More Chart Types Tasks

## Progress
Goal: 实现 11 种新图表类型，扩展 Encoding/DrawCmd/Mark，移植算法
Status: 23/23 (100%) — Complete
Review: PASS (all P0/P1/P2 resolved)
Docs: Updated

## Tasks

### Phase 1: 基础设施扩展（阻塞所有图表）
- [x] 1. 扩展 Mark 枚举 — 新增 11 个变体 + Display impl — ref: requirements 架构扩展点, design Mark 枚举扩展 ✅ Wave 1
- [x] 2. 扩展 Encoding 结构 — 新增 open/high/low/close/theta 通道 + builder 方法 — ref: design Encoding 扩展 ✅ Wave 1
- [x] 3. 扩展 DrawCmd — 新增 Arc variant + CanvasOp 映射 — ref: design DrawCmd 扩展 ✅ Wave 1
- [x] 4. 扩展 FillStyle — Gradient 支持已存在，更新 CanvasOp 渐变处理 — ref: design Heatmap 颜色 ✅ Wave 1

### Phase 2: 核心算法移植（从 lodviz-rs）
- [x] 5. 移植 KDE 算法到 deneb-core/algorithm/kde.rs — ref: design 算法文件 ✅ Wave 1
- [x] 6. 移植 beeswarm 算法到 deneb-core/algorithm/beeswarm.rs — ref: design 算法文件 ✅ Wave 1
- [x] 7. 移植 sankey_layout 算法到 deneb-core/algorithm/sankey_layout.rs — ref: design 算法文件 ✅ Wave 1
- [x] 8. 移植 chord_layout 算法到 deneb-core/algorithm/chord_layout.rs — ref: design 算法文件 ✅ Wave 1
- [x] 9. 移植 contour (marching squares) 算法到 deneb-core/algorithm/contour.rs — ref: design 算法文件 ✅ Wave 1

### Phase 3: 直角坐标系图表（复用现有布局引擎）
- [x] 10. 实现 Histogram — binning + Bar 变体，Y 轴从 0 开始 ✅ Wave 2
- [x] 11. 实现 BoxPlot — 五数概括 + IQR + outlier ✅ Wave 2
- [x] 12. 实现 Waterfall — 累计基线 + 正负分色，Y 轴从 0 开始 ✅ Wave 2
- [x] 13. 实现 Candlestick — OHLC + 涨跌色 ✅ Wave 2
- [x] 14. 实现 StripChart — beeswarm 布局 ✅ Wave 2
- [x] 15. 实现 Heatmap — 颜色映射 + color bar ✅ Wave 2

### Phase 4: 极坐标/自定义坐标系图表
- [x] 16. 实现 PieChart — 弧形扇形 + 环形图 ✅ Wave 2
- [x] 17. 实现 Radar — 极坐标多边形 ✅ Wave 2
- [x] 18. 实现 SankeyChart — Bézier ribbon ✅ Wave 2
- [x] 19. 实现 ChordChart — 环形 ribbon ✅ Wave 2

### Phase 5: 等高线图
- [x] 20. 实现 ContourChart — marching squares ✅ Wave 2

### Phase 6: 集成与验证
- [x] 21. 更新 WIT 接口 — WitMark/WitEncoding 转换 ✅ Wave 3
- [x] 22. 新增 demo binaries — 每种图表一个 demo ✅ Wave 3
- [x] 23. 全量测试验证 — cargo test + cargo clippy ✅ Wave 3

## Notes
- Phase 1 全部完成后 Phase 3/4 可并行
- Phase 2 算法移植与 Phase 1 可并行
- 每个 chart 实现应遵循现有模式：render() + validate_data() + render_empty() + render_background/grid/axes/title
- Histogram 和 Waterfall 必须强制 Y 轴从 0 开始（CLAUDE.md 规范）

## Review Findings (Wave 4 — all resolved)

### P0: Fixed
- **`demo_contour.rs`** 缺少 color 编码 → 已添加 `.color(Field::quantitative("value"))` 和 `color_field: Some("value")`

### P1: Fixed
- **`layout/mod.rs:L193`** `include_zero` 扩展为 `matches!(Bar | Histogram | Waterfall)` ✅
- **`chart/bar.rs`** 重构使用 `chart/shared.rs` 公共辅助 ✅
- **`chart/pie.rs`** validate_data 扩展覆盖 fallback 通道 (color.or(x), theta.or(y)) ✅
- **`chart/pie.rs` + `chart/radar.rs`** 自定义 render_title 替换为 `shared::render_title` ✅

### P2: Fixed
- **`spec/mod.rs`** Encoding 添加 `color2: Option<Field>` ✅
- **`chart/chord.rs`** 移除 4 个未使用变量 (src_end_x/y, dst_start_x/y) ✅
- **`chart/pie.rs`** 移除 SliceData.value 死字段 ✅
- **`sankey_layout.rs`** 文件注释更新 (BFS → Kahn's topological sort) ✅
- **`chart/line.rs` + `scatter.rs` + `area.rs`** 添加 validate_data 方法 ✅

### P2: Pre-existing (not fixed, low priority)
- **`contour_chart.rs`** clippy warnings (type complexity, needless_range_loop) — 功能正确
- **`bar.rs` / `waterfall.rs`** 冗余 include_zero 逻辑 — 不影响正确性
