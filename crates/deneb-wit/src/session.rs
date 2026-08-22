//! 有状态图表会话 — WIT v2 `resource chart` 的实现核心
//!
//! 会话持有数据/主题/交互态/尺寸,宿主经 resource 方法驱动:
//! - `update-data` → 入场相位重置(t 从 0 重播)
//! - `set-state` → hover 即时生效;selected 变更启动状态过渡(t 从 0)
//! - `render(t)` → t 语义由会话当前阶段决定(入场 / 状态过渡 / 稳态)
//! - `hit-regions()` → 稳态几何命中区(datum 附着)
//!
//! 层缓存策略(§design T3):
//! - 入场期:全量重算(层廉价;grid/axis/title 淡入在静态层上表达)
//! - 稳态:全层缓存,t=1 重复请求直接命中缓存
//! - 状态过渡/hover:仅重算 Data 层(Bar);其余 chart 类型回退全量

use crate::convert::{
    chart_output_to_wit_render_result, hit_region_to_wit_hit_region, layer_to_wit_layer,
    wit_chart_spec_with_table, wit_data_table_to_data_table, ConvertError,
};
use crate::wit_types::*;
use deneb_component::theme::{Theme, ThemeRecord, ThemeRecordTheme};
use deneb_component::{
    AreaChart, BarChart, BoxPlotChart, CandlestickChart, ChordChart, ContourChart, HeatmapChart,
    HistogramChart, LineChart, Mark, PieChart, RadarChart, SankeyChart, ScatterChart, StripChart,
    WaterfallChart,
};
use deneb_core::{ChartAnim, Easing, InteractionState, LayerKind, RenderLayers};
use deneb_component::{ChartOutput, Encoding, Field};

/// 静态(稳态)渲染分发 — 15 种 mark
pub fn render_mark_static<T: Theme>(
    spec: &deneb_component::ChartSpec,
    theme: &T,
    data: &deneb_core::DataTable,
) -> Result<ChartOutput, String> {
    use deneb_component::ChartSpec;
    let (spec, theme, data): (&ChartSpec, &T, &deneb_core::DataTable) = (spec, theme, data);
    match spec.mark {
        Mark::Line => LineChart::render(spec, theme, data).map_err(|e| e.to_string()),
        Mark::Bar => BarChart::render(spec, theme, data).map_err(|e| e.to_string()),
        Mark::Scatter => ScatterChart::render(spec, theme, data).map_err(|e| e.to_string()),
        Mark::Area => AreaChart::render(spec, theme, data).map_err(|e| e.to_string()),
        Mark::Pie => PieChart::render(spec, theme, data).map_err(|e| e.to_string()),
        Mark::Histogram => HistogramChart::render(spec, theme, data).map_err(|e| e.to_string()),
        Mark::BoxPlot => BoxPlotChart::render(spec, theme, data).map_err(|e| e.to_string()),
        Mark::Waterfall => WaterfallChart::render(spec, theme, data).map_err(|e| e.to_string()),
        Mark::Candlestick => CandlestickChart::render(spec, theme, data).map_err(|e| e.to_string()),
        Mark::Radar => RadarChart::render(spec, theme, data).map_err(|e| e.to_string()),
        Mark::Heatmap => HeatmapChart::render(spec, theme, data).map_err(|e| e.to_string()),
        Mark::Strip => StripChart::render(spec, theme, data).map_err(|e| e.to_string()),
        Mark::Sankey => SankeyChart::render(spec, theme, data).map_err(|e| e.to_string()),
        Mark::Chord => ChordChart::render(spec, theme, data).map_err(|e| e.to_string()),
        Mark::Contour => ContourChart::render(spec, theme, data).map_err(|e| e.to_string()),
    }
}

/// 动画渲染分发 — Bar 支持 Tier 1 语义动画,其余 mark 忽略 t(最终态)
fn render_mark_anim(
    spec: &deneb_component::ChartSpec,
    theme: &ThemeRecordTheme,
    data: &deneb_core::DataTable,
    anim: &ChartAnim,
) -> Result<ChartOutput, String> {
    match spec.mark {
        Mark::Bar => BarChart::render_with_anim(spec, theme, data, anim).map_err(|e| e.to_string()),
        _ => render_mark_static(spec, theme, data),
    }
}

/// WIT theme record → 内部 ThemeRecord
pub fn wit_theme_to_record(theme: WitTheme) -> ThemeRecord {
    ThemeRecord {
        palette: theme.palette,
        background: theme.background,
        foreground: theme.foreground,
        grid: theme.grid,
        axis: theme.axis,
        title_color: theme.title_color,
        focus_color: theme.focus_color,
        font_family: theme.font_family,
        base_font_size: theme.base_font_size,
        title_font_size: theme.title_font_size,
        label_font_size: theme.label_font_size,
        tick_font_size: theme.tick_font_size,
        margin: theme.margin,
    }
}

/// 入场编排配置(缺省值与 §9.4 对齐)
#[derive(Clone, Copy, Debug)]
struct AnimConfig {
    enter_duration_ms: f64,
    stagger_ms: f64,
    easing: Easing,
    disable: bool,
}

impl Default for AnimConfig {
    fn default() -> Self {
        Self {
            enter_duration_ms: 500.0,
            stagger_ms: 24.0,
            easing: Easing::CubicOut,
            disable: false,
        }
    }
}

impl AnimConfig {
    fn from_wit(cfg: &Option<WitAnimationConfig>) -> Self {
        match cfg {
            None => Self::default(),
            Some(c) => Self {
                enter_duration_ms: c.enter_duration_ms.map(|v| v as f64).unwrap_or(500.0),
                stagger_ms: c.stagger_ms.unwrap_or(24.0),
                easing: Easing::from_str_opt(c.easing.as_deref().unwrap_or("cubic-out")),
                disable: c.disable,
            },
        }
    }
}

/// 图表会话(WIT resource chart 的实现体)
pub struct ChartSession {
    wit_spec: WitChartSpec,
    spec: deneb_component::ChartSpec,
    theme: ThemeRecordTheme,
    table: deneb_core::DataTable,
    anim_cfg: AnimConfig,

    // 阶段状态
    enter_done: bool,
    /// selected 集合变更后置位 — render(t) 的 t 语义切换为状态过渡
    transition_active: bool,
    interaction: InteractionState,
    /// 稳态置灰基线(选中→取消的恢复起点)
    dim_baseline: f64,

    // 缓存
    /// 稳态全层缓存(t=1 且当前 interaction 已结算)
    steady_layers: Option<RenderLayers>,
    /// 稳态命中区(update-data/resize 后重取)
    hit_regions: Vec<WitHitRegion>,
    /// 上次 render 结果缓存(同 t 且无失效时直接返回)
    cached: Option<(f64, WitRenderResult)>,
    /// 上一次返回的结果(dirty = 与上次比较是否变化,供宿主增量翻译)
    last_result: Option<WitRenderResult>,
}

impl ChartSession {
    /// 创建会话(constructor)
    pub fn new(wit_spec: WitChartSpec, theme: Option<WitTheme>) -> Result<Self, String> {
        let anim_cfg = AnimConfig::from_wit(&wit_spec.animation);
        let record = theme.map(wit_theme_to_record).unwrap_or_default();
        let mut session = Self {
            wit_spec,
            // 占位 spec,new() 末尾 rebuild_spec() 立即以真实 spec 覆盖
            spec: deneb_component::ChartSpec::builder()
                .mark(Mark::Bar)
                .encoding(Encoding::new().x(Field::quantitative("x")).y(Field::quantitative("y")))
                .width(1.0)
                .height(1.0)
                .build()
                .map_err(|e| e.to_string())?,
            theme: ThemeRecordTheme::new(record),
            table: deneb_core::DataTable::new(),
            anim_cfg,
            enter_done: false,
            transition_active: false,
            interaction: InteractionState::default(),
            dim_baseline: 1.0,
            steady_layers: None,
            hit_regions: Vec::new(),
            cached: None,
            last_result: None,
        };
        session.rebuild_spec()?;
        Ok(session)
    }

    /// 由 wit_spec(含宽高)重建内部 spec
    fn rebuild_spec(&mut self) -> Result<(), String> {
        self.spec = wit_chart_spec_with_table(&self.wit_spec, &self.table).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 更新数据(预解析表)— 入场重播,缓存全失效
    pub fn set_table(&mut self, table: deneb_core::DataTable) -> Result<(), String> {
        self.table = table;
        self.rebuild_spec()?;
        self.invalidate_all();
        self.enter_done = false;
        self.interaction = InteractionState::default();
        self.dim_baseline = 1.0;
        Ok(())
    }

    /// 更新数据(原始 bytes + format;csv/json 经内部解析,arrow/parquet 由 guest 侧
    /// 经 limpuai 导入解析后调 set_table)
    pub fn update_data(&mut self, data: &[u8], format: &str) -> Result<(), String> {
        let wit_table = crate::lib_mode::parse_data(data, format)?;
        let table = wit_data_table_to_data_table(wit_table).map_err(|e: ConvertError| e.to_string())?;
        self.set_table(table)
    }

    /// 尺寸变化 — 缓存失效(不重播入场)
    pub fn resize(&mut self, width: f64, height: f64) {
        if self.wit_spec.width == width && self.wit_spec.height == height {
            return;
        }
        self.wit_spec.width = width;
        self.wit_spec.height = height;
        if self.rebuild_spec().is_err() {
            return;
        }
        self.steady_layers = None;
        self.cached = None;
        // 命中区随布局变化,重取
        self.refresh_hit_regions();
    }

    /// 交互状态:hover 即时;selected 集合变更 → 状态过渡
    pub fn set_state(&mut self, state: WitInteractionState) {
        let selected: Vec<usize> = state.selected.iter().map(|&i| i as usize).collect();
        let hovered = state.hovered.map(|h| h as usize);
        let selected_changed = self.interaction.selected != selected;
        self.interaction.hovered = hovered;
        self.interaction.selected = selected;
        if selected_changed {
            self.transition_active = true;
        }
        self.cached = None;
    }

    /// 主题更新 — 缓存全失效(入场不重播)
    pub fn set_theme(&mut self, theme: WitTheme) {
        self.theme = ThemeRecordTheme::new(wit_theme_to_record(theme));
        self.steady_layers = None;
        self.cached = None;
        self.refresh_hit_regions();
    }

    /// 渲染(t 语义由会话阶段决定)
    pub fn render(&mut self, t: f64) -> Result<WitRenderResult, String> {
        let mut tt = t.clamp(0.0, 1.0);
        if self.anim_cfg.disable {
            tt = 1.0;
        }

        // 同请求命中缓存
        if let Some((ct, ref result)) = self.cached {
            if (ct - tt).abs() < f64::EPSILON && !self.transition_active && self.enter_done {
                let mut result = result.clone();
                self.apply_dirty_diff(&mut result);
                return Ok(result);
            }
        }

        let mut result = if !self.enter_done {
            self.render_enter(tt)?
        } else {
            // 状态过渡 / hover / 稳态:Data 层重算 + 静态层缓存
            self.render_interaction(tt)?
        };

        self.apply_dirty_diff(&mut result);
        self.cached = Some((tt, result.clone()));
        self.last_result = Some(result.clone());
        Ok(result)
    }

    /// dirty = 与上次返回结果相比是否变化(宿主增量翻译依据)
    fn apply_dirty_diff(&self, result: &mut WitRenderResult) {
        if let Some(last) = &self.last_result {
            for layer in &mut result.layers {
                let prev = last.layers.iter().find(|l| l.kind == layer.kind);
                layer.dirty = match prev {
                    Some(p) => p.commands != layer.commands,
                    None => true,
                };
            }
        }
    }

    /// 入场渲染:全量重算,静态层(grid/axis/title)淡入
    fn render_enter(&mut self, t: f64) -> Result<WitRenderResult, String> {
        let mut anim = self.make_anim();
        anim.enter_t = t;
        // 入场期间交互态即时应用(state_t = 1)
        anim.state_t = 1.0;
        anim.dim_from = if anim.state.has_selection() { anim.dim_alpha } else { 1.0 };

        let mut output = render_mark_anim(&self.spec, &self.theme, &self.table, &anim)?;

        // 静态层淡入(仅未完成时)
        if t < 1.0 {
            let fade = anim.static_fade(t);
            for kind in [LayerKind::Grid, LayerKind::Axis, LayerKind::Title] {
                if let Some(layer) = output.layers.get_layer_mut(kind) {
                    layer.commands.apply_alpha_to_all(fade);
                }
            }
        }

        // 命中区在入场首次渲染时刷新(几何为稳态)
        if self.hit_regions.is_empty() && !output.hit_regions.is_empty() {
            self.hit_regions = output
                .hit_regions
                .iter()
                .map(|r| hit_region_to_wit_hit_region(r.clone()))
                .collect();
        }

        if t >= 1.0 {
            self.enter_done = true;
            self.transition_active = false;
            self.dim_baseline = if self.interaction.has_selection() { 0.32 } else { 1.0 };
            self.steady_layers = Some(output.layers.clone());
        }

        let mut result = chart_output_to_wit_render_result(output, self.theme.default_stroke_width());
        self.attach_peak_emphasis(&mut result);
        Ok(result)
    }

    /// Tier 2 演示附着:单系列 Bar 的峰值柱获得 1.8s 呼吸强调(opacity 1↔0.85,ping-pong)。
    /// 宿主对缓存指令本地插值 — 零 wasm 调用的循环动画主载体示范。
    fn attach_peak_emphasis(&mut self, result: &mut WitRenderResult) {
        if self.anim_cfg.disable {
            return; // 静态场景(ReducedMotion / 宿主显式禁用):不附着循环动画,
                    // 宿主扫描不到 anim-desc → AnimStatus::Done,零持续重绘
        }
        if self.spec.mark != Mark::Bar {
            return;
        }
        if self.spec.encoding.color.is_some() {
            return; // 多系列时命令序与行序不稳定,跳过(MVP 单系列演示)
        }
        // 峰值行:y 列最大数值
        let y_field = match &self.spec.encoding.y {
            Some(f) => f.name.clone(),
            None => return,
        };
        let y_col = match self.table.get_column(&y_field) {
            Some(c) => c,
            None => return,
        };
        let mut peak: Option<(usize, f64)> = None;
        for (i, v) in y_col.values.iter().enumerate() {
            if let Some(n) = v.as_numeric() {
                if peak.is_none() || n > peak.unwrap().1 {
                    peak = Some((i, n));
                }
            }
        }
        let (peak_idx, _) = match peak {
            Some(p) => p,
            None => return,
        };
        let anim = WitAnimDesc {
            property: WitAnimProperty::Opacity,
            keyframes: vec![
                WitKeyframe { t: 0.0, value: 1.0, easing: "ease-in-out".to_string() },
                WitKeyframe { t: 0.5, value: 0.85, easing: "ease-in-out".to_string() },
                WitKeyframe { t: 1.0, value: 1.0, easing: "linear".to_string() },
            ],
            duration_ms: 1800,
            delay_ms: 0,
            loop_mode: WitLoopMode::PingPong,
        };
        if let Some(layer) = result.layers.iter_mut().find(|l| l.kind == "data") {
            if let Some(cmd) = layer.commands.get_mut(peak_idx) {
                cmd.anim = Some(anim);
            }
        }
    }

    /// 交互/稳态渲染:Data 层重算 + 静态层缓存
    fn render_interaction(&mut self, t: f64) -> Result<WitRenderResult, String> {
        let mut anim = self.make_anim();
        anim.enter_t = 1.0;
        if self.transition_active {
            anim.state_t = t;
            anim.dim_from = self.dim_baseline;
        } else {
            anim.state_t = 1.0;
        }

        // 静态层:优先缓存;失效则全量渲染补缓存
        let need_full = self.steady_layers.is_none();
        if need_full {
            let full = render_mark_anim(&self.spec, &self.theme, &self.table, &anim)?;
            if self.hit_regions.is_empty() && !full.hit_regions.is_empty() {
                self.hit_regions = full
                    .hit_regions
                    .iter()
                    .map(|r| hit_region_to_wit_hit_region(r.clone()))
                    .collect();
            }
            // 静态层入缓存(Data 层不入 — 每次交互重算)
            let mut static_layers = full.layers.clone();
            if let Some(data_layer) = static_layers.get_layer_mut(LayerKind::Data) {
                data_layer.clear();
            }
            self.steady_layers = Some(static_layers);
            if t >= 1.0 && self.transition_active {
                self.transition_active = false;
                self.dim_baseline = if anim.state.has_selection() { anim.dim_alpha } else { 1.0 };
            }
            let mut result = chart_output_to_wit_render_result(full, self.theme.default_stroke_width());
            self.attach_peak_emphasis(&mut result);
            return Ok(result);
        }

        // Data 层重算(Bar 专属快路径;其余 mark 全量回退)
        let (data_output, regions) = match self.spec.mark {
            Mark::Bar => BarChart::render_data_layer(&self.spec, &self.theme, &self.table, &anim)
                .map_err(|e| e.to_string())?,
            _ => {
                let full = render_mark_anim(&self.spec, &self.theme, &self.table, &anim)?;
                if t >= 1.0 && self.transition_active {
                    self.transition_active = false;
                    self.dim_baseline = if anim.state.has_selection() { anim.dim_alpha } else { 1.0 };
                    self.steady_layers = Some(full.layers.clone());
                }
                let mut result = chart_output_to_wit_render_result(full, self.theme.default_stroke_width());
                self.attach_peak_emphasis(&mut result);
                return Ok(result);
            }
        };

        if self.hit_regions.is_empty() && !regions.is_empty() {
            self.hit_regions = regions
                .iter()
                .map(|r| hit_region_to_wit_hit_region(r.clone()))
                .collect();
        }

        if t >= 1.0 && self.transition_active {
            self.transition_active = false;
            self.dim_baseline = if anim.state.has_selection() { anim.dim_alpha } else { 1.0 };
        }

        // 组装:缓存的静态层(dirty=false)+ 新 Data 层(dirty=true)
        let stroke_w = self.theme.default_stroke_width();
        let mut layers: Vec<WitLayer> = Vec::new();
        if let Some(cached) = &self.steady_layers {
            for layer in cached.all() {
                layers.push(layer_to_wit_layer(layer.clone(), false, stroke_w));
            }
        }
        // Data 层替换
        if let Some(layer) = layers.iter_mut().find(|l| l.kind == "data") {
            layer.commands = data_output
                .semantic
                .into_iter()
                .flat_map(|c| crate::convert::draw_cmd_to_wit_draw_cmd_flat(c, 0, stroke_w))
                .collect();
            layer.dirty = true;
        } else {
            layers.push(layer_to_wit_layer(
                deneb_core::Layer::with_commands(LayerKind::Data, data_output),
                true,
                stroke_w,
            ));
        }
        layers.sort_by_key(|l| l.z_index);

        let mut result = WitRenderResult { layers };
        self.attach_peak_emphasis(&mut result);
        Ok(result)
    }

    /// 命中区(稳态几何;空表在首次 render 后可用)
    pub fn hit_regions(&self) -> Vec<WitHitRegion> {
        self.hit_regions.clone()
    }

    fn make_anim(&self) -> ChartAnim {
        let record = self.theme.record();
        ChartAnim {
            enter_t: 1.0,
            stagger_ms: self.anim_cfg.stagger_ms,
            enter_duration_ms: self.anim_cfg.enter_duration_ms,
            easing: self.anim_cfg.easing,
            disable: self.anim_cfg.disable,
            state: self.interaction.clone(),
            state_t: 1.0,
            dim_from: 1.0,
            dim_alpha: 0.32,
            outline_color: record
                .focus_color
                .clone()
                .unwrap_or_else(|| record.foreground.clone()),
            outline_width: 1.5,
            hover_boost: 0.08,
        }
    }

    fn invalidate_all(&mut self) {
        self.steady_layers = None;
        self.hit_regions.clear();
        self.cached = None;
    }

    /// 重算命中区(不产出渲染结果 — resize/set_theme 后保持命中区新鲜)
    fn refresh_hit_regions(&mut self) {
        let anim = self.make_anim();
        let regions = match self.spec.mark {
            Mark::Bar => match BarChart::render_data_layer(&self.spec, &self.theme, &self.table, &anim) {
                Ok((_, regions)) => regions,
                Err(_) => return,
            },
            _ => match render_mark_static(&self.spec, &self.theme, &self.table) {
                Ok(output) => output.hit_regions,
                Err(_) => return,
            },
        };
        self.hit_regions = regions
            .iter()
            .map(|r| hit_region_to_wit_hit_region(r.clone()))
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar_spec(width: f64, height: f64) -> WitChartSpec {
        WitChartSpec {
            mark: "bar".to_string(),
            x_field: "category".to_string(),
            y_field: "value".to_string(),
            color_field: None,
            open_field: None,
            high_field: None,
            low_field: None,
            close_field: None,
            theta_field: None,
            size_field: None,
            width,
            height,
            title: Some("Test".to_string()),
            animation: None,
        }
    }

    fn session_with_data() -> ChartSession {
        let mut s = ChartSession::new(bar_spec(400.0, 300.0), None).unwrap();
        s.update_data(b"category,value\nA,10\nB,20\nC,15", "csv").unwrap();
        s
    }

    #[test]
    fn test_enter_animation_grows_bars() {
        let mut s = session_with_data();
        let early = s.render(0.15).unwrap();
        let late = s.render(1.0).unwrap();

        let data_layer = |r: &WitRenderResult| r.layers.iter().find(|l| l.kind == "data").unwrap().clone();
        let height_of = |layer: &WitLayer| -> f64 {
            layer.commands.iter().map(|c| c.params[3]).sum::<f64>()
        };
        let (e, l) = (height_of(&data_layer(&early)), height_of(&data_layer(&late)));
        assert!(e < l, "early={} late={}", e, l);
        assert!(e > 0.0, "部分柱已开始生长");
    }

    #[test]
    fn test_steady_render_cached() {
        let mut s = session_with_data();
        s.render(1.0).unwrap(); // 入场完成
        let r1 = s.render(1.0).unwrap();
        let r2 = s.render(1.0).unwrap();
        // 稳态重复请求:层 dirty 全 false
        assert!(r1.layers.iter().all(|l| !l.dirty));
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_resize_relayouts_geometry_within_new_width() {
        // 宿主实际布局尺寸 < spec 尺寸(容器约束):resize 后标题锚点/柱子
        // 必须按新宽度重排,不得溢出新边界(echodawn 侧标题偏右 bug 的会话层判据)
        let mut s = ChartSession::new(bar_spec(640.0, 360.0), None).unwrap();
        s.update_data(b"category,value\nA,10\nB,20\nC,15\nD,8", "csv").unwrap();
        s.render(1.0).unwrap();

        let title_anchor = |r: &WitRenderResult| -> f64 {
            r.layers
                .iter()
                .flat_map(|l| l.commands.iter())
                .find_map(|c| (c.cmd_type == "text" && c.text_content.as_deref() == Some("Test"))
                    .then_some(c.params[0]))
                .unwrap()
        };
        let max_data_right = |r: &WitRenderResult| -> f64 {
            r.layers
                .iter()
                .filter(|l| l.kind == "data")
                .flat_map(|l| l.commands.iter())
                .filter(|c| c.cmd_type == "rect")
                .map(|c| c.params[0] + c.params[2])
                .fold(0.0, f64::max)
        };

        let before = s.render(1.0).unwrap();
        let before_title = title_anchor(&before);
        let before_right = max_data_right(&before);

        s.resize(560.0, 360.0);
        let after = s.render(1.0).unwrap();
        let after_title = title_anchor(&after);
        let after_right = max_data_right(&after);

        // 标题锚点随新宽度的 plot 中心移动(左移)
        assert!(
            after_title < before_title - 30.0,
            "title anchor should recenter to narrower plot: {before_title} → {after_title}"
        );
        // 柱子几何收敛到新宽度内(560 - margin.right 24 ≈ 536,留容差)
        assert!(
            after_right <= 560.0 - 20.0,
            "bars must fit new width: right={after_right}"
        );
        assert!(before_right > 560.0 - 20.0, "640 布局理应溢出 560 宽(前置自检)");
    }

    #[test]
    fn test_selection_transition_dims_data_layer() {
        let mut s = session_with_data();
        s.render(1.0).unwrap();

        s.set_state(WitInteractionState { hovered: None, selected: vec![1] });
        let mid = s.render(0.5).unwrap();
        let done = s.render(1.0).unwrap();

        let alpha_of = |r: &WitRenderResult| -> Vec<f64> {
            let layer = r.layers.iter().find(|l| l.kind == "data").unwrap();
            layer
                .commands
                .iter()
                .map(|c| match &c.fill {
                    Some(WitPaint::Solid(col)) => {
                        deneb_core::Rgba::parse(col).map(|p| p.a).unwrap_or(1.0)
                    }
                    _ => 1.0,
                })
                .collect()
        };
        let mid_a = alpha_of(&mid);
        let done_a = alpha_of(&done);
        // 中点:非选中项介于 1.0 与 0.32 之间;完成:非选中 = 0.32,选中 = 1.0
        assert!(mid_a.iter().any(|&a| a < 0.999 && a > 0.33), "mid={:?}", mid_a);
        assert!(done_a.contains(&1.0), "选中项保持满色: {:?}", done_a);
        assert!(done_a.iter().any(|&a| (a - 0.32).abs() < 0.01), "置灰 0.32: {:?}", done_a);
    }

    #[test]
    fn test_deselect_recovers() {
        let mut s = session_with_data();
        s.render(1.0).unwrap();
        s.set_state(WitInteractionState { hovered: None, selected: vec![0] });
        s.render(1.0).unwrap();
        // 取消选择 → 从 0.32 恢复到 1.0
        s.set_state(WitInteractionState { hovered: None, selected: vec![] });
        let r = s.render(1.0).unwrap();
        let layer = r.layers.iter().find(|l| l.kind == "data").unwrap();
        for c in &layer.commands {
            if let Some(WitPaint::Solid(col)) = &c.fill {
                let a = deneb_core::Rgba::parse(col).map(|p| p.a).unwrap_or(1.0);
                assert!((a - 1.0).abs() < 0.01, "恢复满色: {}", col);
            }
        }
    }

    #[test]
    fn test_hover_lightens() {
        let mut s = session_with_data();
        s.render(1.0).unwrap();
        s.set_state(WitInteractionState { hovered: Some(0), selected: vec![] });
        let r = s.render(1.0).unwrap();
        let layer = r.layers.iter().find(|l| l.kind == "data").unwrap();
        // hover 提亮:被 hover 的柱(index 0)颜色为 rgba(...) 形式(lighten 输出)
        assert!(matches!(&layer.commands[0].fill, Some(WitPaint::Solid(col)) if col.starts_with("rgba")),
            "commands[0] fill = {:?}", layer.commands[0].fill);
        // 未 hover 的柱保持 hex 原色
        assert!(matches!(&layer.commands[1].fill, Some(WitPaint::Solid(col)) if col.starts_with('#')));
    }

    #[test]
    fn test_hit_regions_carry_datum() {
        let mut s = session_with_data();
        s.render(1.0).unwrap();
        let regions = s.hit_regions();
        assert_eq!(regions.len(), 3);
        assert!(!regions[0].datum.is_empty(), "datum 数据必须过 ABI");
        // datum: [Text("A"), Numeric(10)]
        assert_eq!(regions[0].datum[0], WitFieldValue::Text("A".to_string()));
    }

    #[test]
    fn test_update_data_replays_enter() {
        let mut s = session_with_data();
        s.render(1.0).unwrap();
        s.update_data(b"category,value\nA,5\nB,8\nC,12\nD,20", "csv").unwrap();
        let r = s.render(0.2).unwrap();
        // 入场重播:变化的层 dirty(grid/axis/title 淡入、data 生长);
        // background/legend(空)与上一稳态相同 → clean(宿主可跳过重翻译)
        for l in &r.layers {
            match l.kind.as_str() {
                "data" | "grid" | "axis" | "title" => assert!(l.dirty, "{} 应 dirty", l.kind),
                _ => assert!(!l.dirty, "{} 未变化", l.kind),
            }
        }
        assert_eq!(s.hit_regions().len(), 4);
    }

    #[test]
    fn test_resize_invalidates() {
        let mut s = session_with_data();
        s.render(1.0).unwrap();
        let before = s.hit_regions();
        s.resize(600.0, 400.0);
        let after = s.hit_regions();
        assert!(!after.is_empty());
        // 尺寸变化后命中区宽度应变化(band 变宽)
        let w_before: f64 = before.iter().map(|r| r.bounds_w).sum();
        let w_after: f64 = after.iter().map(|r| r.bounds_w).sum();
        assert!(w_after > w_before, "before={} after={}", w_before, w_after);
    }

    #[test]
    fn test_theme_injection() {
        let theme = WitTheme {
            palette: vec!["#ff0000".to_string()],
            background: "#000000".to_string(),
            foreground: "#eeeeee".to_string(),
            grid: "#222222".to_string(),
            axis: "#444444".to_string(),
            title_color: "#ffffff".to_string(),
            focus_color: Some("#00ff00".to_string()),
            font_family: "TestFont".to_string(),
            base_font_size: 14.0,
            title_font_size: 18.0,
            label_font_size: 12.0,
            tick_font_size: 11.0,
            margin: (20.0, 20.0, 30.0, 40.0),
        };
        let mut s = ChartSession::new(bar_spec(400.0, 300.0), Some(theme)).unwrap();
        s.update_data(b"category,value\nA,10\nB,20", "csv").unwrap();
        let r = s.render(1.0).unwrap();
        // 背景层用注入色
        let bg = r.layers.iter().find(|l| l.kind == "background").unwrap();
        assert!(bg.commands.iter().any(|c| matches!(&c.fill, Some(WitPaint::Solid(col)) if col == "#000000")));

        // set-theme 运行时更新
        let theme2 = WitTheme {
            palette: vec!["#0000ff".to_string()],
            background: "#111111".to_string(),
            foreground: "#eeeeee".to_string(),
            grid: "#222222".to_string(),
            axis: "#444444".to_string(),
            title_color: "#ffffff".to_string(),
            focus_color: None,
            font_family: "TestFont".to_string(),
            base_font_size: 14.0,
            title_font_size: 18.0,
            label_font_size: 12.0,
            tick_font_size: 11.0,
            margin: (20.0, 20.0, 30.0, 40.0),
        };
        s.set_theme(theme2);
        let r2 = s.render(1.0).unwrap();
        let bg2 = r2.layers.iter().find(|l| l.kind == "background").unwrap();
        assert!(bg2.commands.iter().any(|c| matches!(&c.fill, Some(WitPaint::Solid(col)) if col == "#111111")));
    }

    #[test]
    fn test_selected_outline_stroke() {
        let mut s = session_with_data();
        s.render(1.0).unwrap();
        s.set_state(WitInteractionState { hovered: None, selected: vec![2] });
        let r = s.render(1.0).unwrap();
        let layer = r.layers.iter().find(|l| l.kind == "data").unwrap();
        // 选中柱应有 stroke(focus 色)
        assert!(layer.commands.iter().any(|c| c.stroke.is_some()), "选中项 outline");
    }

    #[test]
    fn test_peak_emphasis_tier2_attached() {
        let mut s = session_with_data(); // A=10 B=20 C=15 → 峰值 B(index 1)
        s.render(1.0).unwrap();
        let r = s.render(1.0).unwrap();
        let layer = r.layers.iter().find(|l| l.kind == "data").unwrap();
        let peak = &layer.commands[1];
        let anim = peak.anim.as_ref().expect("峰值柱应携带 Tier 2 anim");
        assert_eq!(anim.property, WitAnimProperty::Opacity);
        assert_eq!(anim.loop_mode, WitLoopMode::PingPong);
        assert!(layer.commands.iter().enumerate().all(|(i, c)| i == 1 || c.anim.is_none()));
    }

    #[test]
    fn test_disable_animation() {
        let spec = WitChartSpec { animation: Some(WitAnimationConfig { disable: true, ..Default::default() }), ..bar_spec(400.0, 300.0) };
        let mut s = ChartSession::new(spec, None).unwrap();
        s.update_data(b"category,value\nA,10\nB,20", "csv").unwrap();
        let r = s.render(0.0).unwrap();
        // disable:t=0 也渲染最终态
        let layer = r.layers.iter().find(|l| l.kind == "data").unwrap();
        assert!(!layer.commands.is_empty());
        // disable:不附着 Tier 2 循环(帧内零 anim-desc,宿主判 Done 零持续重绘)
        assert!(
            !r.layers.iter().any(|l| l.commands.iter().any(|c| c.anim.is_some())),
            "disabled chart must carry no anim-desc"
        );
    }

    #[test]
    fn test_enabled_animation_attaches_tier2_loop() {
        // 对照:未禁用的单系列 bar 附着峰值呼吸(宿主 Continue 的依据)
        let mut s = session_with_data();
        let r = s.render(1.0).unwrap();
        assert!(
            r.layers.iter().any(|l| l.commands.iter().any(|c| c.anim.is_some())),
            "single-series bar should carry peak-emphasis anim-desc"
        );
    }
}
