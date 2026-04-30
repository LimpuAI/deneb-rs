//! 数据解析器模块
//!
//! 提供从 CSV、JSON、Arrow、Parquet 等格式解析数据的能力。

#[cfg(feature = "csv")]
pub mod csv;

#[cfg(feature = "json")]
pub mod json;

#[cfg(feature = "arrow-format")]
pub mod arrow;

#[cfg(feature = "parquet-format")]
pub mod parquet;

// 通用类型
pub use crate::data::DataTable;
