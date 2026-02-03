use crate::error::{Result, XlogError};
use tracing_subscriber::filter::LevelFilter;

/// 日志级别枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Trace 级别 - 最详细
    Trace,
    /// Debug 级别 - 调试信息
    Debug,
    /// Info 级别 - 一般信息
    Info,
    /// Warn 级别 - 警告信息
    Warn,
    /// Error 级别 - 错误信息
    Error,
}

impl Level {
    /// 从字符串解析日志级别（不区分大小写）
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(Self::Trace),
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" | "warning" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            _ => Err(XlogError::InvalidLevel(s.to_string())),
        }
    }
}

impl From<Level> for LevelFilter {
    fn from(level: Level) -> Self {
        match level {
            Level::Trace => Self::TRACE,
            Level::Debug => Self::DEBUG,
            Level::Info => Self::INFO,
            Level::Warn => Self::WARN,
            Level::Error => Self::ERROR,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_from_str() {
        assert_eq!(Level::from_str("trace").unwrap(), Level::Trace);
        assert_eq!(Level::from_str("DEBUG").unwrap(), Level::Debug);
        assert_eq!(Level::from_str("info").unwrap(), Level::Info);
        assert_eq!(Level::from_str("warn").unwrap(), Level::Warn);
        assert_eq!(Level::from_str("warning").unwrap(), Level::Warn);
        assert_eq!(Level::from_str("error").unwrap(), Level::Error);
        assert!(Level::from_str("invalid").is_err());
    }

    #[test]
    fn test_level_ordering() {
        assert!(Level::Trace < Level::Debug);
        assert!(Level::Debug < Level::Info);
        assert!(Level::Info < Level::Warn);
        assert!(Level::Warn < Level::Error);
    }

    #[test]
    fn test_level_to_filter() {
        use tracing_subscriber::filter::LevelFilter;
        assert_eq!(LevelFilter::from(Level::Info), LevelFilter::INFO);
    }
}
