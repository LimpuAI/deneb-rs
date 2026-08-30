//! mark 级 Tier 1 动画实现 — [`MarkAnim`] 的 15 种 mark 落地
//!
//! 设计:渲染器只产稳态指令,入场/状态动画由本模块在 Data 层指令列表上
//! 后处理(几何插值 + 颜色插值),按 `DrawCmd::id`(= 所属 hit-region index)
//! 关联 per-item 语义。Bar 例外:其 Tier 1 动画在 `BarChart::render_with_anim`
//! 内联完成(精确基线生长语义,回归锁定),trait 侧为直通实现。
//!
//! 入场编排统一复用 [`ChartAnim::item_progress`] 的 stagger 窗口
//! (t ∈ [0.1, 1],per-item 级联偏移);pie sweep 用任务指定的 0.02 小相位。

use deneb_core::{
    alpha_fill, brighten_fill, ChartAnim, DrawCmd, FillStyle, InteractionState, MarkAnim,
    PathSegment, StrokeStyle,
};

// ---------------------------------------------------------------------------
// 通用状态应用(hover 提亮 / 选中 outline / 非选中置灰)— 全 mark 共用
// ---------------------------------------------------------------------------

/// per-item 交互渲染态(Bar render_bars 语义的推广):
/// - hover:提亮(theme.hover_color 基准,缺省向白混合)
/// - selected:满色 + hairline outline(真实消费 outline_width)
/// - 有选中时的非选中项:alpha × dim_factor(state_t 插值)
///
/// 仅作用于携带 `id` 的指令;无 id 的系列级指令(折线/面积轮廓)不参与。
pub fn apply_state_generic(
    cmds: &mut Vec<DrawCmd>,
    state: &InteractionState,
    state_t: f64,
    anim: &ChartAnim,
) {
    let dim = anim.dim_factor_at(state_t);
    for cmd in cmds.iter_mut() {
        let id = match cmd {
            DrawCmd::Rect { id, .. }
            | DrawCmd::Path { id, .. }
            | DrawCmd::Circle { id, .. }
            | DrawCmd::Arc { id, .. } => *id,
            _ => continue,
        };
        let Some(id) = id else { continue };
        let hovered = state.hovered == Some(id as usize);
        let selected = state.is_selected(id as usize);

        match cmd {
            DrawCmd::Rect { fill, stroke, .. }
            | DrawCmd::Path { fill, stroke, .. }
            | DrawCmd::Circle { fill, stroke, .. }
            | DrawCmd::Arc { fill, stroke, .. } => {
                if hovered {
                    if let Some(f) = fill.as_mut() {
                        brighten_fill(f, anim);
                    }
                }
                if selected {
                    // outline 优先于既有描边(如 pie 分隔线/box 中位线)
                    *stroke = Some(StrokeStyle::WithWidth {
                        color: anim.outline_color.clone(),
                        width: anim.outline_width,
                    });
                } else if state.has_selection() {
                    if let Some(f) = fill.as_mut() {
                        alpha_fill(f, dim);
                    }
                    // 描边(分隔线等)同步置灰,避免亮线残留
                    match stroke {
                        Some(StrokeStyle::Color(c)) => *c = deneb_core::with_alpha(c, dim),
                        Some(StrokeStyle::WithWidth { color, .. }) => {
                            *color = deneb_core::with_alpha(color, dim)
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// 几何 helpers
// ---------------------------------------------------------------------------

/// 采样路径段长度(Bezier/Quadratic 用控制多边形长度近似 — 动画精度足够)
fn segment_length(prev: (f64, f64), seg: &PathSegment) -> f64 {
    match seg {
        PathSegment::LineTo(x, y) => dist(prev, (*x, *y)),
        PathSegment::BezierTo(cp1x, cp1y, cp2x, cp2y, x, y) => {
            dist(prev, (*cp1x, *cp1y)) + dist((*cp1x, *cp1y), (*cp2x, *cp2y)) + dist((*cp2x, *cp2y), (*x, *y))
        }
        PathSegment::QuadraticTo(cpx, cpy, x, y) => {
            dist(prev, (*cpx, *cpy)) + dist((*cpx, *cpy), (*x, *y))
        }
        PathSegment::Arc(_, _, r, start, end, _) => (end - start).abs() * r.max(0.0),
        PathSegment::MoveTo(..) | PathSegment::Close => 0.0,
    }
}

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

fn seg_endpoint(prev: (f64, f64), seg: &PathSegment) -> (f64, f64) {
    match seg {
        PathSegment::MoveTo(x, y)
        | PathSegment::LineTo(x, y)
        | PathSegment::BezierTo(_, _, _, _, x, y)
        | PathSegment::QuadraticTo(_, _, x, y) => (*x, *y),
        _ => prev,
    }
}

/// 折线按累计弧长前缀揭示:保留 MoveTo,截断点落在段内线性插值
/// (Bezier 段近似沿弦插值 — 动画用途足够)。
fn truncate_by_arclength(segments: &mut Vec<PathSegment>, fraction: f64) {
    if fraction >= 1.0 || segments.len() < 2 {
        return;
    }
    if fraction <= 0.0 {
        // 完全未开始:仅保留起点(零长路径 — 宿主绘制不可见)
        segments.truncate(1);
        return;
    }

    // 累计长度
    let mut cur = (0.0, 0.0);
    let mut total = 0.0;
    for seg in segments.iter() {
        total += segment_length(cur, seg);
        cur = seg_endpoint(cur, seg);
    }
    if total <= 0.0 {
        return;
    }
    let target = total * fraction.clamp(0.0, 1.0);

    let mut kept: Vec<PathSegment> = Vec::with_capacity(segments.len());
    let mut cur = (0.0, 0.0);
    let mut acc = 0.0;
    let mut truncated = false;
    for seg in segments.iter() {
        if truncated {
            break;
        }
        match seg {
            PathSegment::MoveTo(x, y) => {
                kept.push(seg.clone());
                cur = (*x, *y);
            }
            _ => {
                let len = segment_length(cur, seg);
                if acc + len >= target && len > 0.0 {
                    // 在本段内截断
                    let f = ((target - acc) / len).clamp(0.0, 1.0);
                    let end = seg_endpoint(cur, seg);
                    let cut = (cur.0 + (end.0 - cur.0) * f, cur.1 + (end.1 - cur.1) * f);
                    kept.push(PathSegment::LineTo(cut.0, cut.1));
                    truncated = true;
                } else {
                    acc += len;
                    kept.push(seg.clone());
                    cur = seg_endpoint(cur, seg);
                }
            }
        }
    }
    *segments = kept;
}

/// 路径全部顶点(含 Bezier/Quadratic 控制点 — 变换用途)
fn path_points(segments: &[PathSegment]) -> Vec<(f64, f64)> {
    let mut pts = Vec::new();
    for seg in segments {
        match seg {
            PathSegment::MoveTo(x, y) | PathSegment::LineTo(x, y) => pts.push((*x, *y)),
            PathSegment::BezierTo(cp1x, cp1y, cp2x, cp2y, x, y) => {
                pts.push((*cp1x, *cp1y));
                pts.push((*cp2x, *cp2y));
                pts.push((*x, *y));
            }
            PathSegment::QuadraticTo(cpx, cpy, x, y) => {
                pts.push((*cpx, *cpy));
                pts.push((*x, *y));
            }
            PathSegment::Arc(cx, cy, r, start, end, _) => {
                // 端点 + 中点采样,保证弧形顶点参与
                for i in 0..=2 {
                    let a = start + (end - start) * i as f64 / 2.0;
                    pts.push((cx + r * a.cos(), cy + r * a.sin()));
                }
            }
            PathSegment::Close => {}
        }
    }
    pts
}

/// 对路径所有坐标应用映射 f: (x, y) -> (x', y')
fn map_path_points(segments: &mut Vec<PathSegment>, f: &dyn Fn(f64, f64) -> (f64, f64)) {
    for seg in segments.iter_mut() {
        match seg {
            PathSegment::MoveTo(x, y) | PathSegment::LineTo(x, y) => {
                let (nx, ny) = f(*x, *y);
                *x = nx;
                *y = ny;
            }
            PathSegment::BezierTo(cp1x, cp1y, cp2x, cp2y, x, y) => {
                let (a, b) = f(*cp1x, *cp1y);
                let (c, d) = f(*cp2x, *cp2y);
                let (e, g) = f(*x, *y);
                *cp1x = a; *cp1y = b; *cp2x = c; *cp2y = d; *x = e; *y = g;
            }
            PathSegment::QuadraticTo(cpx, cpy, x, y) => {
                let (a, b) = f(*cpx, *cpy);
                let (c, d) = f(*x, *y);
                *cpx = a; *cpy = b; *x = c; *y = d;
            }
            PathSegment::Arc(..) | PathSegment::Close => {}
        }
    }
}

/// per-item eased 局部进度(item_progress 的缓动封装)
fn local_progress(anim: &ChartAnim, item: usize, t: f64) -> f64 {
    anim.easing.apply(anim.item_progress(item, t))
}

/// 阶段窗淡入:t ∈ [start, 1] 线性到 1
fn phase_fade(t: f64, start: f64) -> f64 {
    if t <= start {
        0.0
    } else {
        ((t - start) / (1.0 - start)).min(1.0)
    }
}

// ---------------------------------------------------------------------------
// 15 种 mark 实现
// ---------------------------------------------------------------------------

/// Bar — 直通实现(参考实现的动画在渲染器内联,回归锁定)
pub enum BarAnim {}
impl MarkAnim for BarAnim {
    fn apply_enter(_cmds: &mut Vec<DrawCmd>, _t: f64, _anim: &ChartAnim) {}
    fn apply_state(_cmds: &mut Vec<DrawCmd>, _state: &InteractionState, _state_t: f64, _anim: &ChartAnim) {}
}

/// Line — draw-on:按累计弧长前缀揭示;stagger 按系列(每 Path = 一系列);
/// 入场期 [0, 0.15] 相位整体淡入;单点退化 Circle 走 drop-in。
pub enum LineAnim {}
impl MarkAnim for LineAnim {
    fn apply_enter(cmds: &mut Vec<DrawCmd>, t: f64, anim: &ChartAnim) {
        if anim.disable || t >= 1.0 {
            return;
        }
        // 入场期 [0, 0.15] 相位整体淡入
        let fade = (t / 0.15).clamp(0.0, 1.0);
        let mut series_ordinal = 0usize;
        for cmd in cmds.iter_mut() {
            match cmd {
                DrawCmd::Path { segments, stroke, .. } => {
                    let p = local_progress(anim, series_ordinal, t);
                    truncate_by_arclength(segments, p);
                    if let Some(StrokeStyle::Color(c)) = stroke {
                        *c = deneb_core::with_alpha(c, fade);
                    }
                    series_ordinal += 1;
                }
                DrawCmd::Circle { r, fill, id, .. } => {
                    // 顶点标记(T26):按点 id(行序)drop-in
                    let idx = id.map(|i| i as usize).unwrap_or(0);
                    let p = local_progress(anim, idx, t);
                    drop_in_point(r, fill, p);
                }
                _ => {}
            }
        }
    }

    fn apply_state(cmds: &mut Vec<DrawCmd>, state: &InteractionState, state_t: f64, anim: &ChartAnim) {
        apply_state_generic(cmds, state, state_t, anim);
    }
}

/// Area — rise:数据点向基线(y 最大值,面积闭合底边)比例生长 +
/// 整体 [0, 0.3] 淡入;stagger 按系列。
pub enum AreaAnim {}
impl MarkAnim for AreaAnim {
    fn apply_enter(cmds: &mut Vec<DrawCmd>, t: f64, anim: &ChartAnim) {
        if anim.disable || t >= 1.0 {
            return;
        }
        let fade = (t / 0.3).clamp(0.0, 1.0);
        let mut series_ordinal = 0usize;
        for cmd in cmds.iter_mut() {
            match cmd {
                DrawCmd::Path { segments, fill, stroke, .. } => {
                    let p = local_progress(anim, series_ordinal, t);
                    series_ordinal += 1;
                    if p <= 0.0 {
                        segments.clear();
                        continue;
                    }
                    if p < 1.0 {
                        // 基线 = 路径最低边(像素 y 最大处)
                        let baseline = path_points(segments).iter().fold(f64::NEG_INFINITY, |m, q| m.max(q.1));
                        map_path_points(segments, &|x, y| (x, baseline + (y - baseline) * p));
                    }
                    if let Some(f) = fill.as_mut() {
                        alpha_fill(f, fade);
                    }
                    if let Some(StrokeStyle::Color(c)) = stroke {
                        *c = deneb_core::with_alpha(c, fade);
                    }
                }
                // 面积顶点标记(T26 顶点圆点):按点 id drop-in
                DrawCmd::Circle { r, fill, id, .. } => {
                    let idx = id.map(|i| i as usize).unwrap_or(0);
                    let p = local_progress(anim, idx, t);
                    drop_in_point(r, fill, p);
                }
                DrawCmd::Rect { y, height, fill, .. } => {
                    // 单点退化矩形:从底边生长
                    let p = local_progress(anim, series_ordinal, t);
                    series_ordinal += 1;
                    if p < 1.0 {
                        let new_h = *height * p;
                        *y += *height - new_h; // 底边锚定:顶边随生长上移
                        *height = new_h;
                    }
                    if let Some(f) = fill.as_mut() {
                        alpha_fill(f, fade);
                    }
                }
                _ => {}
            }
        }
    }

    fn apply_state(cmds: &mut Vec<DrawCmd>, state: &InteractionState, state_t: f64, anim: &ChartAnim) {
        apply_state_generic(cmds, state, state_t, anim);
    }
}

/// Pie — sweep:扇形 end-angle 自 start 插值(12 点方向顺时针展开);
/// per-slice 0.02 小相位级联;标签随所属扇形进度淡入。
pub enum PieAnim {}
impl MarkAnim for PieAnim {
    fn apply_enter(cmds: &mut Vec<DrawCmd>, t: f64, anim: &ChartAnim) {
        if anim.disable || t >= 1.0 {
            return;
        }
        let mut slice_ordinal = 0usize;
        let mut last_p = 0.0;
        for cmd in cmds.iter_mut() {
            match cmd {
                DrawCmd::Arc { start_angle, end_angle, fill, stroke, .. } => {
                    // 数据项生长窗 [0.1, 1] + 0.02/slice 相位偏移
                    let phase = 0.1 + 0.02 * slice_ordinal as f64;
                    let p = anim.easing.apply(phase_fade(t, phase));
                    last_p = p;
                    slice_ordinal += 1;
                    if p <= 0.0 {
                        // 未开始:压成零扇形(不可见)
                        *end_angle = *start_angle;
                        continue;
                    }
                    if p < 1.0 {
                        let span = *end_angle - *start_angle;
                        *end_angle = *start_angle + span * p;
                    }
                    if let Some(f) = fill.as_mut() {
                        alpha_fill(f, p.max(0.0));
                    }
                    if let Some(StrokeStyle::Color(c)) = stroke {
                        *c = deneb_core::with_alpha(c, p.max(0.0));
                    }
                }
                DrawCmd::Text { style, .. } => {
                    // 标签随其后扇形的进度淡入(指令序:Arc 后跟其标签)
                    if last_p < 1.0 {
                        if let FillStyle::Color(c) = &mut style.fill {
                            *c = deneb_core::with_alpha(c, last_p.max(0.0));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn apply_state(cmds: &mut Vec<DrawCmd>, state: &InteractionState, state_t: f64, anim: &ChartAnim) {
        apply_state_generic(cmds, state, state_t, anim);
    }
}

/// 散点族(scatter/strip)— drop-in:per-point scale 0.3→1 + opacity 0→1,
/// 围绕点中心(Circle r 缩放);stagger 按 id(行序)。
pub enum ScatterAnim {}
impl MarkAnim for ScatterAnim {
    fn apply_enter(cmds: &mut Vec<DrawCmd>, t: f64, anim: &ChartAnim) {
        if anim.disable || t >= 1.0 {
            return;
        }
        for cmd in cmds.iter_mut() {
            if let DrawCmd::Circle { r, fill, id, .. } = cmd {
                let idx = id.map(|i| i as usize).unwrap_or(0);
                let p = local_progress(anim, idx, t);
                drop_in_point(r, fill, p);
            }
        }
    }

    fn apply_state(cmds: &mut Vec<DrawCmd>, state: &InteractionState, state_t: f64, anim: &ChartAnim) {
        apply_state_generic(cmds, state, state_t, anim);
    }
}

/// Sankey — link flow:link 带 alpha 渐入 + 宽度 30%→100% 生长(向带域
/// 垂直中心收拢);node 恒定。stagger 按 link 序。
pub enum SankeyAnim {}
impl MarkAnim for SankeyAnim {
    fn apply_enter(cmds: &mut Vec<DrawCmd>, t: f64, anim: &ChartAnim) {
        if anim.disable || t >= 1.0 {
            return;
        }
        let mut link_ordinal = 0usize;
        for cmd in cmds.iter_mut() {
            if let DrawCmd::Path { segments, fill, stroke, .. } = cmd {
                let p = local_progress(anim, link_ordinal, t);
                link_ordinal += 1;
                if p <= 0.0 {
                    segments.clear();
                    continue;
                }
                if p < 1.0 {
                    // 宽度生长:向带域垂直中心收拢(y 方向 squeeze),x 恒定
                    let pts = path_points(segments);
                    let min_y = pts.iter().fold(f64::INFINITY, |m, q| m.min(q.1));
                    let max_y = pts.iter().fold(f64::NEG_INFINITY, |m, q| m.max(q.1));
                    let cy = (min_y + max_y) / 2.0;
                    let w = 0.3 + 0.7 * p;
                    map_path_points(segments, &|x, y| (x, cy + (y - cy) * w));
                }
                if let Some(f) = fill.as_mut() {
                    alpha_fill(f, p.max(0.0));
                }
                if let Some(StrokeStyle::Color(c)) = stroke {
                    *c = deneb_core::with_alpha(c, p.max(0.0));
                }
            }
        }
    }

    fn apply_state(cmds: &mut Vec<DrawCmd>, state: &InteractionState, state_t: f64, anim: &ChartAnim) {
        apply_state_generic(cmds, state, state_t, anim);
    }
}

/// 柱状族(histogram/waterfall/candlestick/box_plot)— 从基线生长:
/// 柱体矩形底边锚定高度缩放;附属线族(影线/须/中位线/连接线)随同相位淡入;
/// stagger 按 id(命中区序)。
pub enum BarFamilyAnim {}
impl MarkAnim for BarFamilyAnim {
    fn apply_enter(cmds: &mut Vec<DrawCmd>, t: f64, anim: &ChartAnim) {
        if anim.disable || t >= 1.0 {
            return;
        }
        for cmd in cmds.iter_mut() {
            match cmd {
                DrawCmd::Rect { y, height, fill, id, .. } => {
                    let idx = id.map(|i| i as usize).unwrap_or(0);
                    let p = local_progress(anim, idx, t);
                    if p <= 0.0 {
                        *height = 0.0;
                        continue;
                    }
                    if p < 1.0 {
                        let new_h = *height * p;
                        *y += *height - new_h; // 底边锚定:顶边随生长上移
                        *height = new_h;
                    }
                    if let Some(f) = fill.as_mut() {
                        alpha_fill(f, p.max(0.05));
                    }
                }
                DrawCmd::Path { stroke, id, .. } => {
                    // 影线/须/中位线:同相位淡入(不缩放几何 — 线族缩放易产生错位)
                    let idx = id.map(|i| i as usize).unwrap_or(0);
                    let p = local_progress(anim, idx, t);
                    if p < 1.0 {
                        if let Some(StrokeStyle::Color(c)) = stroke {
                            *c = deneb_core::with_alpha(c, p.max(0.0));
                        }
                    }
                }
                DrawCmd::Circle { r, fill, id, .. } => {
                    // box_plot 异常值点
                    let idx = id.map(|i| i as usize).unwrap_or(0);
                    let p = local_progress(anim, idx, t);
                    drop_in_point(r, fill, p);
                }
                _ => {}
            }
        }
    }

    fn apply_state(cmds: &mut Vec<DrawCmd>, state: &InteractionState, state_t: f64, anim: &ChartAnim) {
        apply_state_generic(cmds, state, state_t, anim);
    }
}

/// Radar — 半径比例展开:系列多边形顶点向质心(图表中心)比例缩放;
/// 顶点圆点随 drop-in;stagger 按系列。
pub enum RadarAnim {}
impl MarkAnim for RadarAnim {
    fn apply_enter(cmds: &mut Vec<DrawCmd>, t: f64, anim: &ChartAnim) {
        if anim.disable || t >= 1.0 {
            return;
        }
        let mut series_ordinal = 0usize;
        for cmd in cmds.iter_mut() {
            match cmd {
                DrawCmd::Path { segments, fill, stroke, id: _, .. } => {
                    let p = local_progress(anim, series_ordinal, t);
                    series_ordinal += 1;
                    if p <= 0.0 {
                        segments.clear();
                        continue;
                    }
                    if p < 1.0 {
                        let pts = path_points(segments);
                        if !pts.is_empty() {
                            let n = pts.len() as f64;
                            let cx = pts.iter().map(|q| q.0).sum::<f64>() / n;
                            let cy = pts.iter().map(|q| q.1).sum::<f64>() / n;
                            map_path_points(segments, &|x, y| (cx + (x - cx) * p, cy + (y - cy) * p));
                        }
                    }
                    if let Some(f) = fill.as_mut() {
                        alpha_fill(f, p.max(0.0));
                    }
                    if let Some(StrokeStyle::Color(c)) = stroke {
                        *c = deneb_core::with_alpha(c, p.max(0.0));
                    }
                }
                DrawCmd::Circle { r, fill, id, .. } => {
                    let idx = id.map(|i| i as usize).unwrap_or(series_ordinal.saturating_sub(1));
                    let p = local_progress(anim, idx, t);
                    drop_in_point(r, fill, p);
                }
                _ => {}
            }
        }
    }

    fn apply_state(cmds: &mut Vec<DrawCmd>, state: &InteractionState, state_t: f64, anim: &ChartAnim) {
        apply_state_generic(cmds, state, state_t, anim);
    }
}

/// Chord — opacity 淡入(ribbon 与弧段分相位:ribbon 先行,弧段随后)
pub enum ChordAnim {}
impl MarkAnim for ChordAnim {
    fn apply_enter(cmds: &mut Vec<DrawCmd>, t: f64, anim: &ChartAnim) {
        fade_all(cmds, t, anim, 0.1, 0.15);
    }

    fn apply_state(cmds: &mut Vec<DrawCmd>, state: &InteractionState, state_t: f64, anim: &ChartAnim) {
        apply_state_generic(cmds, state, state_t, anim);
    }
}

/// Contour — opacity 淡入 + 等值线路径按序微级联(半径/带宽生长在开放
/// 等值线上无自然锚点,淡入为较自然者)
pub enum ContourAnim {}
impl MarkAnim for ContourAnim {
    fn apply_enter(cmds: &mut Vec<DrawCmd>, t: f64, anim: &ChartAnim) {
        fade_all(cmds, t, anim, 0.1, 0.04);
    }

    fn apply_state(cmds: &mut Vec<DrawCmd>, state: &InteractionState, state_t: f64, anim: &ChartAnim) {
        apply_state_generic(cmds, state, state_t, anim);
    }
}

/// Heatmap — 单元格按行序(stagger via id)淡入
pub enum HeatmapAnim {}
impl MarkAnim for HeatmapAnim {
    fn apply_enter(cmds: &mut Vec<DrawCmd>, t: f64, anim: &ChartAnim) {
        if anim.disable || t >= 1.0 {
            return;
        }
        for cmd in cmds.iter_mut() {
            if let DrawCmd::Rect { fill, stroke, id, .. } = cmd {
                let idx = id.map(|i| i as usize).unwrap_or(0);
                let p = local_progress(anim, idx, t);
                if p < 1.0 {
                    if let Some(f) = fill.as_mut() {
                        alpha_fill(f, p.max(0.0));
                    }
                    if let Some(StrokeStyle::Color(c)) = stroke {
                        *c = deneb_core::with_alpha(c, p.max(0.0));
                    }
                }
            }
        }
    }

    fn apply_state(cmds: &mut Vec<DrawCmd>, state: &InteractionState, state_t: f64, anim: &ChartAnim) {
        apply_state_generic(cmds, state, state_t, anim);
    }
}

// ---------------------------------------------------------------------------
// 共用小工具
// ---------------------------------------------------------------------------

/// 点 drop-in:r × (0.3 + 0.7p),alpha = p(围绕点中心缩放)
fn drop_in_point(r: &mut f64, fill: &mut Option<FillStyle>, p: f64) {
    if p >= 1.0 {
        return;
    }
    *r *= 0.3 + 0.7 * p.max(0.0);
    if let Some(f) = fill.as_mut() {
        alpha_fill(f, p.max(0.0));
    }
}

/// 全指令淡入(带 per-item 级联小相位);Shape/Path 全覆盖
fn fade_all(cmds: &mut Vec<DrawCmd>, t: f64, anim: &ChartAnim, window_start: f64, cascade: f64) {
    if anim.disable || t >= 1.0 {
        return;
    }
    for (i, cmd) in cmds.iter_mut().enumerate() {
        let p = anim.easing.apply(phase_fade(t, window_start + cascade * i as f64));
        match cmd {
            DrawCmd::Rect { fill, stroke, .. }
            | DrawCmd::Path { fill, stroke, .. }
            | DrawCmd::Circle { fill, stroke, .. }
            | DrawCmd::Arc { fill, stroke, .. } => {
                if p < 1.0 {
                    if let Some(f) = fill.as_mut() {
                        alpha_fill(f, p.max(0.0));
                    }
                    if let Some(StrokeStyle::Color(c)) = stroke {
                        *c = deneb_core::with_alpha(c, p.max(0.0));
                    }
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Mark 分发(session 调度入口)
// ---------------------------------------------------------------------------

use crate::spec::Mark;

/// 按 mark 分发入场动画(15 种全覆盖;Bar 直通)
pub fn apply_enter_for(mark: &Mark, cmds: &mut Vec<DrawCmd>, t: f64, anim: &ChartAnim) {
    match mark {
        Mark::Bar => BarAnim::apply_enter(cmds, t, anim),
        Mark::Line => LineAnim::apply_enter(cmds, t, anim),
        Mark::Area => AreaAnim::apply_enter(cmds, t, anim),
        Mark::Pie => PieAnim::apply_enter(cmds, t, anim),
        Mark::Scatter => ScatterAnim::apply_enter(cmds, t, anim),
        Mark::Strip => ScatterAnim::apply_enter(cmds, t, anim),
        Mark::Histogram | Mark::Waterfall | Mark::Candlestick | Mark::BoxPlot => {
            BarFamilyAnim::apply_enter(cmds, t, anim)
        }
        Mark::Radar => RadarAnim::apply_enter(cmds, t, anim),
        Mark::Sankey => SankeyAnim::apply_enter(cmds, t, anim),
        Mark::Chord => ChordAnim::apply_enter(cmds, t, anim),
        Mark::Contour => ContourAnim::apply_enter(cmds, t, anim),
        Mark::Heatmap => HeatmapAnim::apply_enter(cmds, t, anim),
    }
}

/// 按 mark 分发状态过渡(所有 mark 经由 generic per-item 状态应用)
pub fn apply_state_for(
    mark: &Mark,
    cmds: &mut Vec<DrawCmd>,
    state: &InteractionState,
    state_t: f64,
    anim: &ChartAnim,
) {
    match mark {
        Mark::Bar => BarAnim::apply_state(cmds, state, state_t, anim),
        Mark::Line => LineAnim::apply_state(cmds, state, state_t, anim),
        Mark::Area => AreaAnim::apply_state(cmds, state, state_t, anim),
        Mark::Pie => PieAnim::apply_state(cmds, state, state_t, anim),
        Mark::Scatter | Mark::Strip => ScatterAnim::apply_state(cmds, state, state_t, anim),
        Mark::Histogram | Mark::Waterfall | Mark::Candlestick | Mark::BoxPlot => {
            BarFamilyAnim::apply_state(cmds, state, state_t, anim)
        }
        Mark::Radar => RadarAnim::apply_state(cmds, state, state_t, anim),
        Mark::Sankey => SankeyAnim::apply_state(cmds, state, state_t, anim),
        Mark::Chord => ChordAnim::apply_state(cmds, state, state_t, anim),
        Mark::Contour => ContourAnim::apply_state(cmds, state, state_t, anim),
        Mark::Heatmap => HeatmapAnim::apply_state(cmds, state, state_t, anim),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deneb_core::FillStyle;

    fn rect(id: Option<u32>, y: f64, h: f64) -> DrawCmd {
        DrawCmd::Rect {
            x: 0.0, y, width: 10.0, height: h,
            fill: Some(FillStyle::Color("#1f77b4".to_string())),
            stroke: None,
            corner_radius: None,
            corner_radii: None,
            id,
        }
    }

    fn circle(id: Option<u32>, r: f64) -> DrawCmd {
        DrawCmd::Circle {
            cx: 5.0, cy: 5.0, r,
            fill: Some(FillStyle::Color("#1f77b4".to_string())),
            stroke: None,
            id,
        }
    }

    fn alpha_of(cmd: &DrawCmd) -> f64 {
        match cmd {
            DrawCmd::Rect { fill: Some(FillStyle::Color(c)), .. }
            | DrawCmd::Circle { fill: Some(FillStyle::Color(c)), .. } => {
                deneb_core::Rgba::parse(c).map(|p| p.a).unwrap_or(1.0)
            }
            _ => 1.0,
        }
    }

    #[test]
    fn bar_family_grows_from_bottom() {
        let anim = ChartAnim::steady();
        let mut cmds = vec![rect(Some(0), 10.0, 100.0), rect(Some(1), 20.0, 50.0)];
        BarFamilyAnim::apply_enter(&mut cmds, 0.5, &anim);
        let (y0, h0) = match &cmds[0] {
            DrawCmd::Rect { y, height, .. } => (*y, *height),
            _ => unreachable!(),
        };
        // 底边锚定:y + h 不变(= 110)
        assert!((y0 + h0 - 110.0).abs() < 1e-9, "y={} h={}", y0, h0);
        assert!(h0 > 0.0 && h0 < 100.0, "部分生长 h={}", h0);
        // t=0:高度 0
        let mut cmds = vec![rect(Some(0), 10.0, 100.0)];
        BarFamilyAnim::apply_enter(&mut cmds, 0.0, &anim);
        assert!(matches!(&cmds[0], DrawCmd::Rect { height: h, .. } if *h == 0.0));
    }

    #[test]
    fn scatter_drop_in_scales_radius() {
        let anim = ChartAnim::steady();
        let mut cmds = vec![circle(Some(0), 10.0)];
        ScatterAnim::apply_enter(&mut cmds, 0.0, &anim);
        assert!(matches!(&cmds[0], DrawCmd::Circle { r, .. } if (*r - 3.0).abs() < 1e-9), "r 应为 30%");
        let mut cmds = vec![circle(Some(0), 10.0)];
        ScatterAnim::apply_enter(&mut cmds, 1.0, &anim);
        assert!(matches!(&cmds[0], DrawCmd::Circle { r, .. } if (*r - 10.0).abs() < 1e-9), "t=1 直通");
    }

    #[test]
    fn state_generic_dims_and_outlines() {
        let mut anim = ChartAnim::steady();
        anim.state = InteractionState { hovered: None, selected: vec![0] };
        anim.state_t = 1.0;
        let mut cmds = vec![rect(Some(0), 0.0, 10.0), rect(Some(1), 0.0, 10.0)];
        apply_state_generic(&mut cmds, &anim.state, 1.0, &anim);
        // 选中项:outline stroke(消费 outline_width 1.5)
        assert!(matches!(&cmds[0], DrawCmd::Rect { stroke: Some(StrokeStyle::WithWidth { width, .. }), .. } if (*width - 1.5).abs() < 1e-9));
        // 非选中:置灰 0.32
        assert!((alpha_of(&cmds[1]) - 0.32).abs() < 0.01, "alpha={}", alpha_of(&cmds[1]));
        // 无 id 指令不参与
        let mut cmds = vec![rect(None, 0.0, 10.0)];
        apply_state_generic(&mut cmds, &anim.state, 1.0, &anim);
        assert!((alpha_of(&cmds[0]) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn state_generic_hover_brightens() {
        let mut anim = ChartAnim::steady();
        anim.state = InteractionState { hovered: Some(0), selected: vec![] };
        let mut cmds = vec![rect(Some(0), 0.0, 10.0)];
        apply_state_generic(&mut cmds, &anim.state, 1.0, &anim);
        match &cmds[0] {
            DrawCmd::Rect { fill: Some(FillStyle::Color(c)), .. } => {
                assert!(c.starts_with("rgba"), "hover 提亮输出 rgba: {}", c);
            }
            _ => panic!("expected solid fill"),
        }
        // hover_color 语义色通道:向语义色混合
        anim.hover_color = Some("#ff0000".to_string());
        let mut cmds = vec![rect(Some(0), 0.0, 10.0)];
        apply_state_generic(&mut cmds, &anim.state, 1.0, &anim);
        match &cmds[0] {
            DrawCmd::Rect { fill: Some(FillStyle::Color(c)), .. } => {
                let rgba = deneb_core::Rgba::parse(c).unwrap();
                // #1f77b4 向 #ff0000 混 8%:r 上移
                assert!(rgba.r > 0x1f, "r={} 应向红端偏移", rgba.r);
            }
            _ => panic!("expected solid fill"),
        }
    }

    #[test]
    fn line_truncates_by_arclength() {
        let mut segments = vec![
            PathSegment::MoveTo(0.0, 0.0),
            PathSegment::LineTo(10.0, 0.0),
            PathSegment::LineTo(20.0, 0.0),
        ];
        truncate_by_arclength(&mut segments, 0.5);
        assert_eq!(segments.len(), 2, "半程截断在第二段内");
        match &segments[1] {
            PathSegment::LineTo(x, _) => assert!((*x - 10.0).abs() < 1e-9),
            _ => panic!("expected LineTo"),
        }
        // 0%:仅 MoveTo
        let mut segments = vec![
            PathSegment::MoveTo(0.0, 0.0),
            PathSegment::LineTo(10.0, 0.0),
        ];
        truncate_by_arclength(&mut segments, 0.0);
        assert_eq!(segments.len(), 1);
    }

    #[test]
    fn pie_sweep_interpolates_end_angle() {
        let anim = ChartAnim::steady();
        let mk = |end: f64| DrawCmd::Arc {
            cx: 0.0, cy: 0.0, r: 10.0,
            start_angle: -std::f64::consts::FRAC_PI_2,
            end_angle: end,
            fill: Some(FillStyle::Color("#1f77b4".to_string())),
            stroke: None,
            id: Some(0),
        };
        let full_end = -std::f64::consts::FRAC_PI_2 + 1.0;
        let mut cmds = vec![mk(full_end)];
        PieAnim::apply_enter(&mut cmds, 0.0, &anim);
        match &cmds[0] {
            DrawCmd::Arc { start_angle, end_angle, .. } => {
                assert!((end_angle - start_angle).abs() < 1e-9, "t=0 零扇形");
            }
            _ => unreachable!(),
        }
        let mut cmds = vec![mk(full_end)];
        PieAnim::apply_enter(&mut cmds, 1.0, &anim);
        match &cmds[0] {
            DrawCmd::Arc { end_angle, .. } => {
                assert!((end_angle - full_end).abs() < 1e-9, "t=1 直通");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn disable_is_noop() {
        let mut anim = ChartAnim::steady();
        anim.disable = true;
        let mut cmds = vec![rect(Some(0), 0.0, 10.0)];
        BarFamilyAnim::apply_enter(&mut cmds, 0.0, &anim);
        assert!(matches!(&cmds[0], DrawCmd::Rect { height: h, .. } if *h == 10.0), "disable 稳态直通");
    }
}
