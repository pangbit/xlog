use std::io;

/// xlog 的统一错误类型
#[derive(Debug, thiserror::Error)]
pub enum XlogError {
    /// 日志系统初始化失败
    #[error("Failed to initialize logger: {0}")]
    InitFailed(String),

    /// 日志系统已经被初始化
    #[error("Logger already initialized")]
    AlreadyInitialized,

    /// IO 错误
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// 无效的日志级别
    #[error("Invalid log level: {0}")]
    InvalidLevel(String),

    /// 无效的配置
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// xlog Result 类型别名
pub type Result<T> = std::result::Result<T, XlogError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_error_display() {
        let err = XlogError::InvalidLevel("invalid".to_string());
        assert_eq!(err.to_string(), "Invalid log level: invalid");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let xlog_err: XlogError = io_err.into();
        assert!(matches!(xlog_err, XlogError::Io(_)));
    }
}
