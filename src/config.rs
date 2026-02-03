use crate::error::Result;
use crate::formatter::Formatter;
use crate::level::Level;
use crate::output::Output;
use std::collections::HashMap;
use tracing_appender::non_blocking::WorkerGuard;

/// 速率限制配置
#[derive(Debug, Clone, Copy)]
pub struct RateLimit {
    pub(crate) max_per_second: u32,
}

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
    pub(crate) module_filters: HashMap<String, Level>,
    pub(crate) custom_directives: Vec<String>,
    pub(crate) rate_limit: Option<RateLimit>,
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
            module_filters: HashMap::new(),
            custom_directives: Vec::new(),
            rate_limit: None,
        }
    }
}

impl Config {
    /// 创建新的配置（使用默认值）
    #[must_use]
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

    /// 设置模块的日志级别
    pub fn filter_module(mut self, module: &str, level: Level) -> Self {
        self.module_filters.insert(module.to_string(), level);
        self
    }

    /// 添加自定义过滤指令
    pub fn add_directive(mut self, directive: &str) -> Self {
        self.custom_directives.push(directive.to_string());
        self
    }

    /// 设置速率限制
    pub fn rate_limit(mut self, max_per_second: u32) -> Self {
        self.rate_limit = Some(RateLimit { max_per_second });
        self
    }

    /// 初始化日志系统
    pub fn init(self) -> Result<WorkerGuard> {
        crate::init::initialize(self)
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

#[cfg(test)]
mod advanced_tests {
    use super::*;

    #[test]
    fn test_config_with_module_filters() {
        let mut filters = HashMap::new();
        filters.insert("hyper".to_string(), Level::Warn);

        let config = Config {
            module_filters: filters,
            ..Default::default()
        };

        assert_eq!(config.module_filters.len(), 1);
        assert_eq!(config.module_filters.get("hyper"), Some(&Level::Warn));
    }

    #[test]
    fn test_filter_module_api() {
        let config = Config::new()
            .filter_module("hyper", Level::Warn)
            .filter_module("tokio", Level::Error);

        assert_eq!(config.module_filters.get("hyper"), Some(&Level::Warn));
        assert_eq!(config.module_filters.get("tokio"), Some(&Level::Error));
    }

    #[test]
    fn test_add_directive_api() {
        let config = Config::new()
            .add_directive("hyper::client=trace")
            .add_directive("tokio::runtime=debug");

        assert_eq!(config.custom_directives.len(), 2);
        assert_eq!(config.custom_directives[0], "hyper::client=trace");
    }
}
