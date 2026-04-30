//! Core 错误类型定义
//!
//! 提供 deneb-core 的统一错误类型，不依赖 thiserror crate。

use std::fmt;

/// 数据格式类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataFormat {
    /// CSV 格式
    Csv,
    /// JSON 格式
    Json,
    /// Arrow 格式
    Arrow,
    /// Parquet 格式
    Parquet,
}

impl fmt::Display for DataFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataFormat::Csv => write!(f, "CSV"),
            DataFormat::Json => write!(f, "JSON"),
            DataFormat::Arrow => write!(f, "Arrow"),
            DataFormat::Parquet => write!(f, "Parquet"),
        }
    }
}

/// Core 错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum CoreError {
    /// 解析错误
    ParseError {
        /// 错误来源信息
        source: String,
        /// 数据格式
        format: DataFormat,
    },
    /// 编码错误
    InvalidEncoding {
        /// 字段名
        field: String,
        /// 错误原因
        reason: String,
    },
    /// 比例尺错误
    ScaleError {
        /// 错误原因
        reason: String,
    },
    /// 数据为空
    EmptyData,
    /// 无效输入
    InvalidInput {
        /// 错误原因
        reason: String,
    },
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::ParseError { source, format } => {
                write!(f, "Failed to parse {} data: {}", format, source)
            }
            CoreError::InvalidEncoding { field, reason } => {
                write!(f, "Invalid encoding for field '{}': {}", field, reason)
            }
            CoreError::ScaleError { reason } => {
                write!(f, "Scale error: {}", reason)
            }
            CoreError::EmptyData => {
                write!(f, "Data is empty")
            }
            CoreError::InvalidInput { reason } => {
                write!(f, "Invalid input: {}", reason)
            }
        }
    }
}

impl std::error::Error for CoreError {}

// 辅助构造函数
impl CoreError {
    /// 创建解析错误
    pub fn parse_error(source: impl Into<String>, format: DataFormat) -> Self {
        CoreError::ParseError {
            source: source.into(),
            format,
        }
    }

    /// 创建编码错误
    pub fn invalid_encoding(field: impl Into<String>, reason: impl Into<String>) -> Self {
        CoreError::InvalidEncoding {
            field: field.into(),
            reason: reason.into(),
        }
    }

    /// 创建比例尺错误
    pub fn scale_error(reason: impl Into<String>) -> Self {
        CoreError::ScaleError {
            reason: reason.into(),
        }
    }

    /// 创建空数据错误
    pub fn empty_data() -> Self {
        CoreError::EmptyData
    }

    /// 创建无效输入错误
    pub fn invalid_input(reason: impl Into<String>) -> Self {
        CoreError::InvalidInput {
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_format_display() {
        assert_eq!(DataFormat::Csv.to_string(), "CSV");
        assert_eq!(DataFormat::Json.to_string(), "JSON");
        assert_eq!(DataFormat::Arrow.to_string(), "Arrow");
        assert_eq!(DataFormat::Parquet.to_string(), "Parquet");
    }

    #[test]
    fn test_core_error_display() {
        let err = CoreError::parse_error("invalid syntax", DataFormat::Csv);
        assert_eq!(err.to_string(), "Failed to parse CSV data: invalid syntax");

        let err = CoreError::invalid_encoding("price", "not a number");
        assert_eq!(err.to_string(), "Invalid encoding for field 'price': not a number");

        let err = CoreError::scale_error("domain is empty");
        assert_eq!(err.to_string(), "Scale error: domain is empty");

        let err = CoreError::empty_data();
        assert_eq!(err.to_string(), "Data is empty");

        let err = CoreError::invalid_input("width cannot be negative");
        assert_eq!(err.to_string(), "Invalid input: width cannot be negative");
    }

    #[test]
    fn test_core_error_equality() {
        let err1 = CoreError::parse_error("syntax error", DataFormat::Json);
        let err2 = CoreError::parse_error("syntax error", DataFormat::Json);
        assert_eq!(err1, err2);

        let err3 = CoreError::parse_error("other error", DataFormat::Json);
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_core_error_helpers() {
        let err = CoreError::parse_error("test", DataFormat::Csv);
        matches!(err, CoreError::ParseError { .. });

        let err = CoreError::invalid_encoding("field", "reason");
        matches!(err, CoreError::InvalidEncoding { .. });

        let err = CoreError::scale_error("reason");
        matches!(err, CoreError::ScaleError { .. });

        let err = CoreError::empty_data();
        matches!(err, CoreError::EmptyData);

        let err = CoreError::invalid_input("reason");
        matches!(err, CoreError::InvalidInput { .. });
    }
}
