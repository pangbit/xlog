use crate::level::Level;
use crate::output::Output;
use crate::formatter::Formatter;
use crate::error::Result;
use tracing_appender::non_blocking::WorkerGuard;

/// 日志系统配置
#[derive(Debug)]
pub struct Config {
    /// 日志级别
    pub(crate) level: Level,
    /// 输出目标
    pub(crate) output: Output,
    /// 格式化器
    pub(crate) formatter: Formatter,
    /// 是否包含文件名
    pub(crate) include_file: bool,
    /// 是否包含行号
    pub(crate) include_line_number: bool,
    /// 是否包含 target
    pub(crate) include_target: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            level: Level::Info,
            output: Output::Stdout,
            formatter: Formatter::Pretty,
            include_file: true,
            include_line_number: true,
            include_target: false,
        }
    }
}

impl Config {
    /// 创建新的配置（使用默认值）
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置日志级别
    pub fn level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    /// 设置输出目标
    pub fn output(mut self, output: Output) -> Self {
        self.output = output;
        self
    }

    /// 设置格式化器
    pub fn formatter(mut self, formatter: Formatter) -> Self {
        self.formatter = formatter;
        self
    }

    /// 设置是否包含文件名
    pub fn with_file(mut self, enabled: bool) -> Self {
        self.include_file = enabled;
        self
    }

    /// 设置是否包含行号
    pub fn with_line_number(mut self, enabled: bool) -> Self {
        self.include_line_number = enabled;
        self
    }

    /// 设置是否包含 target
    pub fn with_target(mut self, enabled: bool) -> Self {
        self.include_target = enabled;
        self
    }

    /// 初始化日志系统
    pub fn init(self) -> Result<WorkerGuard> {
        // Temporarily unimplemented until init module is created
        unimplemented!("init module not yet implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.level, Level::Info);
        assert!(matches!(config.output, Output::Stdout));
        assert!(matches!(config.formatter, Formatter::Pretty));
        assert!(config.include_file);
        assert!(config.include_line_number);
        assert!(!config.include_target);
    }

    #[test]
    fn test_config_builder() {
        let config = Config::new()
            .level(Level::Debug)
            .formatter(Formatter::Json)
            .with_target(true);

        assert_eq!(config.level, Level::Debug);
        assert!(matches!(config.formatter, Formatter::Json));
        assert!(config.include_target);
    }

    #[test]
    fn test_config_fluent_api() {
        let config = Config::new()
            .level(Level::Warn)
            .output(Output::stderr())
            .with_file(false)
            .with_line_number(false);

        assert_eq!(config.level, Level::Warn);
        assert!(!config.include_file);
        assert!(!config.include_line_number);
    }
}
