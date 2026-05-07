//! 算法模块
//!
//! 提供数据处理算法，包括降采样、核密度估计、蜂群布局、桑基图布局、弦图布局、等高线等。

pub mod beeswarm;
pub mod chord_layout;
pub mod contour;
pub mod downsample;
pub mod kde;
pub mod sankey_layout;

pub use beeswarm::{beeswarm_layout, StripLayout};
pub use chord_layout::{layout_chord, ChordLayout, ChordNode, ChordRibbon};
pub use contour::{close_open_path_at_boundary, marching_squares, ContourPath};
pub use downsample::{lttb, m4};
pub use kde::gaussian_kde;
pub use sankey_layout::{
    layout_sankey, SankeyLayout, SankeyLink, SankeyLinkInput, SankeyNode, SankeyNodeInput,
};
