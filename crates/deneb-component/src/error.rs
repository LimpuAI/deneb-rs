//! deneb-component 错误类型定义

use std::fmt;

/// Component 错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentError {
    /// 无效配置
    InvalidConfig {
        /// 错误原因
        reason: String,
    },
    /// Core 错误包装
    Core(deneb_core::CoreError),
}

impl fmt::Display for ComponentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComponentError::InvalidConfig { reason } => {
                write!(f, "Invalid configuration: {}", reason)
            }
            ComponentError::Core(err) => write!(f, "Core error: {}", err),
        }
    }
}

impl std::error::Error for ComponentError {}

// 从 CoreError 转换
impl From<deneb_core::CoreError> for ComponentError {
    fn from(err: deneb_core::CoreError) -> Self {
        ComponentError::Core(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_error_display() {
        let err = ComponentError::InvalidConfig {
            reason: "mark is required".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid configuration: mark is required"
        );
    }

    #[test]
    fn test_component_error_from_core() {
        let core_err = deneb_core::CoreError::empty_data();
        let comp_err: ComponentError = core_err.into();
        assert_eq!(comp_err.to_string(), "Core error: Data is empty");
    }
}
