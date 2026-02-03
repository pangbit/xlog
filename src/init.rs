use crate::config::Config;
use crate::error::{Result, XlogError};
use crate::formatter::Formatter;
use crate::level::Level;
use crate::output::Output;
use crate::rate_limiter::RateLimiter;
use std::sync::{Arc, Once};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;
use tracing::{Event, Subscriber};

static INIT: Once = Once::new();
static mut INITIALIZED: bool = false;

/// 速率限制层 - 包装另一个层并应用速率限制
struct RateLimitLayer<L> {
    inner: L,
    limiter: Arc<RateLimiter>,
}

impl<S, L> Layer<S> for RateLimitLayer<L>
where
    L: Layer<S>,
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        // 检查速率限制
        if self.limiter.try_acquire() {
            self.inner.on_event(event, ctx);
        }
        // 超过限制则静默丢弃
    }
}

/// 初始化日志系统
pub(crate) fn initialize(config: Config) -> Result<WorkerGuard> {
    let mut result: Option<Result<WorkerGuard>> = None;

    INIT.call_once(|| {
        result = Some(do_initialize(config));
        unsafe {
            INITIALIZED = true;
        }
    });

    // 如果已经初始化过，返回错误
    if unsafe { INITIALIZED } && result.is_none() {
        return Err(XlogError::AlreadyInitialized);
    }

    result.unwrap()
}

fn build_env_filter(config: &Config) -> Result<EnvFilter> {
    let mut directives = vec![level_to_str(&config.level).to_string()];

    // 添加模块过滤
    for (module, level) in &config.module_filters {
        directives.push(format!("{}={}", module, level_to_str(level)));
    }

    // 添加自定义指令
    directives.extend(config.custom_directives.clone());

    // 构建过滤字符串
    let filter_str = directives.join(",");

    // 解析过滤器
    EnvFilter::try_new(&filter_str)
        .map_err(|e| XlogError::InvalidConfig(e.to_string()))
}

fn level_to_str(level: &Level) -> &'static str {
    match level {
        Level::Trace => "trace",
        Level::Debug => "debug",
        Level::Info => "info",
        Level::Warn => "warn",
        Level::Error => "error",
    }
}

fn do_initialize(config: Config) -> Result<WorkerGuard> {
    // 使用新的 build_env_filter 替代原有逻辑
    let env_filter = build_env_filter(&config)?;

    // 保存速率限制配置（如果有）
    let rate_limit = config.rate_limit;

    // 保存输出类型信息（用于判断是否需要 ANSI 颜色）
    let is_terminal_output = matches!(config.output, Output::Stdout | Output::Stderr);

    // 构建输出 writer
    let (non_blocking, guard) = match config.output {
        Output::Stdout => tracing_appender::non_blocking(std::io::stdout()),
        Output::Stderr => tracing_appender::non_blocking(std::io::stderr()),
        Output::File {
            path,
            rotation,
            max_files,
        } => {
            let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
                .rotation(rotation.into())
                .max_log_files(max_files)
                .build(path)
                .map_err(|e| XlogError::InitFailed(e.to_string()))?;
            tracing_appender::non_blocking(file_appender)
        }
        Output::Multi(_) => {
            // 多输出需要更复杂的实现，暂时使用 stdout
            tracing_appender::non_blocking(std::io::stdout())
        }
    };

    // 根据格式化器类型初始化不同的订阅者
    match config.formatter {
        Formatter::Compact => {
            let fmt_layer = fmt::layer()
                .with_writer(non_blocking)
                .with_target(config.include_target)
                .with_file(config.include_file)
                .with_line_number(config.include_line_number)
                .with_timer(fmt::time::LocalTime::rfc_3339())
                .with_ansi(is_terminal_output)
                .compact();

            // 应用速率限制
            if let Some(rate_limit) = rate_limit {
                let limiter = Arc::new(RateLimiter::new(rate_limit.max_per_second));
                let rate_limited_layer = RateLimitLayer {
                    inner: fmt_layer,
                    limiter,
                };

                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(rate_limited_layer)
                    .try_init()
                    .map_err(|e| XlogError::InitFailed(e.to_string()))?;
            } else {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer)
                    .try_init()
                    .map_err(|e| XlogError::InitFailed(e.to_string()))?;
            }
        }
        Formatter::Pretty => {
            let fmt_layer = fmt::layer()
                .with_writer(non_blocking)
                .with_target(config.include_target)
                .with_file(config.include_file)
                .with_line_number(config.include_line_number)
                .with_timer(fmt::time::LocalTime::rfc_3339())
                .with_ansi(is_terminal_output)
                .pretty();

            // 应用速率限制
            if let Some(rate_limit) = rate_limit {
                let limiter = Arc::new(RateLimiter::new(rate_limit.max_per_second));
                let rate_limited_layer = RateLimitLayer {
                    inner: fmt_layer,
                    limiter,
                };

                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(rate_limited_layer)
                    .try_init()
                    .map_err(|e| XlogError::InitFailed(e.to_string()))?;
            } else {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer)
                    .try_init()
                    .map_err(|e| XlogError::InitFailed(e.to_string()))?;
            }
        }
        Formatter::Json => {
            let fmt_layer = fmt::layer()
                .with_writer(non_blocking)
                .with_target(config.include_target)
                .with_file(config.include_file)
                .with_line_number(config.include_line_number)
                .with_timer(fmt::time::LocalTime::rfc_3339())
                .with_ansi(false) // JSON doesn't need ANSI colors
                .json();

            // 应用速率限制
            if let Some(rate_limit) = rate_limit {
                let limiter = Arc::new(RateLimiter::new(rate_limit.max_per_second));
                let rate_limited_layer = RateLimitLayer {
                    inner: fmt_layer,
                    limiter,
                };

                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(rate_limited_layer)
                    .try_init()
                    .map_err(|e| XlogError::InitFailed(e.to_string()))?;
            } else {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer)
                    .try_init()
                    .map_err(|e| XlogError::InitFailed(e.to_string()))?;
            }
        }
    }

    Ok(guard)
}
