// deneb-component: Chart 类型实现层
//
// 本 crate 基于 deneb-core 提供的基础类型，实现各种具体的图表类型（柱状图、折线图等）。

#![warn(missing_docs)]
#![warn(clippy::all)]

//! # deneb-component
//!
//! Chart 类型实现层。
//!
//! 本 crate 基于 deneb-core 提供的基础类型，实现各种具体的图表类型（柱状图、折线图等）。

pub mod error;
pub mod spec;
pub mod theme;
pub mod layout;
pub mod chart;

pub use error::ComponentError;
pub use spec::{Mark, Field, Encoding, Aggregate, ChartSpec, ChartSpecBuilder};
pub use theme::{Theme, Margin, LayoutConfig, DefaultTheme, DarkTheme, ForestTheme, NordicTheme, CappuccinoTheme};
pub use layout::{LayoutResult, PlotArea, AxisLayout, Orientation, TickCalculator, compute_layout};
pub use chart::{ChartOutput, LineChart, BarChart, ScatterChart, AreaChart, BoxPlotChart, StripChart, HistogramChart, WaterfallChart, CandlestickChart, HeatmapChart, PieChart, RadarChart, SankeyChart, ChordChart, ContourChart};

// 重新导出 deneb-core 的类型
pub use deneb_core;
