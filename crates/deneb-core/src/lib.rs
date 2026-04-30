// deneb-core: 纯计算核心，提供数据类型、渲染指令、比例尺等基础能力
//
// 本 crate 是 deneb-rs 可视化库的核心，不依赖任何前端框架或运行时，
// 仅负责数据转换和 Canvas 2D 指令生成。

#![warn(missing_docs)]
#![warn(clippy::all)]

//! # deneb-core
//!
//! 纯计算核心，提供数据类型、渲染指令、比例尺等基础能力。
//!
//! 本 crate 是 deneb-rs 可视化库的核心，不依赖任何前端框架或运行时，
//! 仅负责数据转换和 Canvas 2D 指令生成。


pub mod data;
pub mod style;
pub mod instruction;
pub mod layer;
pub mod scale;
pub mod error;
pub mod parser;
pub mod algorithm;
pub mod interaction;

// 重新导出常用类型
pub use data::{DataTable, Column, FieldValue, DataType, Schema};
pub use style::{FillStyle, StrokeStyle, Gradient, GradientKind, GradientStop};
pub use style::{TextStyle, FontWeight, FontStyle, TextAnchor, TextBaseline};
pub use instruction::{DrawCmd, PathSegment, CanvasOp, RenderOutput};
pub use layer::{LayerKind, Layer, RenderLayers};
pub use scale::{Scale, ScaleDomain, ScaleRange, LinearScale, LogScale, TimeScale, OrdinalScale, BandScale};
pub use error::{CoreError, DataFormat};
pub use parser::*;
pub use algorithm::*;
pub use interaction::{HitRegion, BoundingBox, HitResult};
pub use interaction::{CoordLookup, SimpleLookup};
