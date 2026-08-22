//! 动画支持类型 — Tier 1 语义动画输入
//!
//! 图表渲染的动画相位由宿主驱动(t ∈ [0,1]),提供方按 t 插值图表语义
//! (柱高生长、stagger 级联、高亮/置灰过渡)。Tier 2 参数动画见 WIT anim-desc
//! (宿主本地插值,不经此模块)。

/// 缓动函数
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Easing {
    /// 线性
    Linear,
    /// 三次缓入
    CubicIn,
    /// 三次缓出(入场默认)
    CubicOut,
    /// 三次缓入缓出
    CubicInOut,
}

impl Easing {
    /// 应用缓动,t ∈ [0,1](自动钳位)
    pub fn apply(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::CubicIn => t * t * t,
            Easing::CubicOut => 1.0 - (1.0 - t).powi(3),
            Easing::CubicInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
        }
    }

    /// 从字符串解析(协议 easing 字段)
    pub fn from_str_opt(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "linear" => Easing::Linear,
            "cubic-in" => Easing::CubicIn,
            "cubic-in-out" => Easing::CubicInOut,
            // 缺省 cubic-out(入场业界标准,D3/ECharts)
            _ => Easing::CubicOut,
        }
    }
}

/// 交互状态(hover 即时生效,selected 由 state_t 插值过渡)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InteractionState {
    /// hover 的 datum 行索引
    pub hovered: Option<usize>,
    /// 选中的 datum 行索引集合(空 = 无选中)
    pub selected: Vec<usize>,
}

impl InteractionState {
    /// 是否有选中项(决定其余项是否置灰)
    pub fn has_selection(&self) -> bool {
        !self.selected.is_empty()
    }

    /// 指定行是否被选中
    pub fn is_selected(&self, row: usize) -> bool {
        self.selected.contains(&row)
    }
}

/// 图表语义动画输入(Tier 1)
///
/// 稳态(无动画)用 [`ChartAnim::steady`]。
#[derive(Clone, Debug)]
pub struct ChartAnim {
    /// 入场相位 t ∈ [0,1](数据项 stagger 生长)
    pub enter_t: f64,
    /// 每项级联相位偏移(毫秒)
    pub stagger_ms: f64,
    /// 入场总时长(毫秒,stagger 归一化基准)
    pub enter_duration_ms: f64,
    /// 数据项缓动
    pub easing: Easing,
    /// 动画禁用(ReducedMotion / 静态场景)→ 一切按最终态渲染
    pub disable: bool,
    /// 交互状态
    pub state: InteractionState,
    /// 状态过渡相位 t ∈ [0,1](置灰/outline 强度)
    pub state_t: f64,
    /// 状态过渡起点置灰强度(上一稳态;取消选择时从置灰恢复)
    pub dim_from: f64,
    /// 非选中项置灰目标透明度(ECharts blur 范式 0.32)
    pub dim_alpha: f64,
    /// 选中项 outline 颜色
    pub outline_color: String,
    /// 选中项 outline 宽度(逻辑像素)
    pub outline_width: f64,
    /// hover 项提亮幅度(0-1,向白色混合的比例)
    pub hover_boost: f64,
}

impl ChartAnim {
    /// 稳态(无动画、无交互)
    pub fn steady() -> Self {
        Self {
            enter_t: 1.0,
            stagger_ms: 24.0,
            enter_duration_ms: 500.0,
            easing: Easing::CubicOut,
            disable: false,
            state: InteractionState::default(),
            state_t: 1.0,
            dim_from: 1.0,
            dim_alpha: 0.32,
            outline_color: "#333333".to_string(),
            outline_width: 1.5,
            hover_boost: 0.08,
        }
    }

    /// stagger 级联总量上限(毫秒)— 保证尾部等待有界
    pub const STAGGER_CAP_MS: f64 = 400.0;

    /// 数据项 i 的生长相位:全局 t 映射到该项的局部进度 [0,1]。
    ///
    /// 项 i 的窗口起点 = 数据生长区起点(0.1)+ 级联偏移占比 × 剩余跨度,
    /// 所有项在 t=1 处同时完成。
    pub fn item_progress(&self, item_index: usize, t: f64) -> f64 {
        if self.disable {
            return 1.0;
        }
        let t = t.clamp(0.0, 1.0);
        let grow_start = 0.1;
        let grow_span = 1.0 - grow_start;
        let offset = (item_index as f64 * self.stagger_ms).min(Self::STAGGER_CAP_MS);
        let total = Self::STAGGER_CAP_MS.max(self.stagger_ms);
        let item_start = grow_start + grow_span * (offset / total) * 0.8;
        if t <= item_start {
            0.0
        } else {
            ((t - item_start) / (1.0 - item_start)).min(1.0)
        }
    }

    /// 静态层(grid/axis/title)入场淡入进度:窗口 t ∈ [0, 0.3]
    pub fn static_fade(&self, t: f64) -> f64 {
        if self.disable {
            return 1.0;
        }
        self.easing.apply((t.clamp(0.0, 1.0) / 0.3).min(1.0))
    }

    /// 状态过渡当前置灰强度:lerp(dim_from, target, ease(state_t))。
    /// target = 有选择 ? dim_alpha : 1.0;dim_from 由会话层提供(上一稳态强度),
    /// 保证「选中→取消」也能平滑恢复。
    pub fn dim_factor(&self) -> f64 {
        let target = if self.state.has_selection() {
            self.dim_alpha
        } else {
            1.0
        };
        if self.disable {
            return target;
        }
        let e = self.easing.apply(self.state_t.clamp(0.0, 1.0));
        lerp(self.dim_from, target, e)
    }
}

/// 线性插值
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_bounds() {
        for e in [Easing::Linear, Easing::CubicIn, Easing::CubicOut, Easing::CubicInOut] {
            assert!((e.apply(0.0) - 0.0).abs() < 1e-9);
            assert!((e.apply(1.0) - 1.0).abs() < 1e-9);
            assert!((e.apply(-1.0) - 0.0).abs() < 1e-9);
            assert!((e.apply(2.0) - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn test_item_progress_stagger() {
        let anim = ChartAnim {
            enter_duration_ms: 500.0,
            stagger_ms: 24.0,
            ..ChartAnim::steady()
        };
        // 首项:窗口起点 0.1,在 t=0.55 处约完成一半
        let p0 = anim.item_progress(0, 0.55);
        assert!(p0 > 0.4 && p0 < 0.6, "p0={}", p0);
        // 后续项起点更晚
        let p5 = anim.item_progress(5, 0.55);
        assert!(p5 < p0, "p5={} p0={}", p5, p0);
        // 所有项在 t=1 完成
        for i in [0usize, 3, 7, 20] {
            assert!((anim.item_progress(i, 1.0) - 1.0).abs() < 1e-9);
        }
        // t=0 全部为零
        assert_eq!(anim.item_progress(0, 0.0), 0.0);
    }

    #[test]
    fn test_item_progress_disable() {
        let anim = ChartAnim {
            disable: true,
            ..ChartAnim::steady()
        };
        assert_eq!(anim.item_progress(9, 0.0), 1.0);
        assert_eq!(anim.static_fade(0.0), 1.0);
    }

    #[test]
    fn test_dim_factor() {
        let mut anim = ChartAnim::steady();
        anim.dim_from = 1.0;
        anim.state.selected = vec![1];
        anim.state_t = 1.0;
        assert!((anim.dim_factor() - 0.32).abs() < 1e-9);
        anim.state_t = 0.0;
        assert!((anim.dim_factor() - 1.0).abs() < 1e-9);
        // 取消选择:从置灰(0.32)恢复
        anim.dim_from = 0.32;
        anim.state.selected = vec![];
        anim.state_t = 1.0;
        assert!((anim.dim_factor() - 1.0).abs() < 1e-9);
        anim.state_t = 0.0;
        assert!((anim.dim_factor() - 0.32).abs() < 1e-9);
    }
}
