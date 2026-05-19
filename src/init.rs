use crate::config::{Config, RateLimit};
use crate::error::{Result, XlogError};
use crate::formatter::Formatter;
use crate::level::Level;
use crate::output::{Output, OutputWithFormatter};
use crate::rate_limiter::RateLimiter;
use std::sync::{Arc, Once};
use tracing::{Event, Subscriber};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::Context;
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;
use tracing_subscriber::{fmt, EnvFilter};

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
    EnvFilter::try_new(&filter_str).map_err(|e| XlogError::InvalidConfig(e.to_string()))
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
    // Build environment filter
    let env_filter = build_env_filter(&config)?;

    // Save rate limit configuration
    let rate_limit = config.rate_limit;

    // Dispatch to single or multi output initialization
    match config.output {
        Output::Multi(ref outputs) => {
            initialize_multi_output(outputs.clone(), env_filter, &config, rate_limit)
        }
        ref output => {
            let formatter = config.formatter;
            initialize_single_output(output.clone(), formatter, env_filter, &config, rate_limit)
        }
    }
}

fn build_file_appender(
    path: impl AsRef<std::path::Path>,
    rotation: crate::output::Rotation,
    max_files: usize,
    filename_prefix: Option<&str>,
) -> Result<tracing_appender::rolling::RollingFileAppender> {
    let mut builder = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(rotation.into())
        .max_log_files(max_files);
    if let Some(prefix) = filename_prefix {
        builder = builder.filename_prefix(prefix);
    }
    builder
        .build(path)
        .map_err(|e| XlogError::InitFailed(e.to_string()))
}

/// MultiGuard inner structure for managing multiple WorkerGuards
struct MultiGuardInner {
    _guards: Vec<WorkerGuard>,
}

impl Drop for MultiGuardInner {
    fn drop(&mut self) {
        // All guards drop automatically in reverse order
        // This ensures proper cleanup of all output streams
    }
}

/// Initialize multi-output logging
fn initialize_multi_output(
    outputs: Vec<OutputWithFormatter>,
    env_filter: EnvFilter,
    config: &Config,
    rate_limit_cfg: Option<RateLimit>,
) -> Result<WorkerGuard> {
    if outputs.is_empty() {
        return Err(XlogError::InvalidConfig(
            "Multi output requires at least one output".to_string(),
        ));
    }

    // Handle based on number of outputs
    match outputs.len() {
        1 => initialize_single_multi(outputs, env_filter, config, rate_limit_cfg),
        2 => initialize_dual_multi(outputs, env_filter, config, rate_limit_cfg),
        3 => initialize_triple_multi(outputs, env_filter, config, rate_limit_cfg),
        _ => Err(XlogError::InvalidConfig(
            "Currently supports up to 3 outputs in multi-output mode".to_string(),
        )),
    }
}

/// Initialize with single output in multi mode
fn initialize_single_multi(
    outputs: Vec<OutputWithFormatter>,
    env_filter: EnvFilter,
    config: &Config,
    rate_limit_cfg: Option<RateLimit>,
) -> Result<WorkerGuard> {
    use tracing_subscriber::layer::SubscriberExt;

    let mut iter = outputs.into_iter();
    let output1 = iter.next().unwrap();

    let (nb1, g1, f1) = match output1 {
        OutputWithFormatter::Stdout { formatter } => {
            let (nb, g) = tracing_appender::non_blocking(std::io::stdout());
            (nb, g, formatter)
        }
        OutputWithFormatter::Stderr { formatter } => {
            let (nb, g) = tracing_appender::non_blocking(std::io::stderr());
            (nb, g, formatter)
        }
        OutputWithFormatter::File {
            path,
            rotation,
            max_files,
            filename_prefix,
            formatter,
        } => {
            let fa = build_file_appender(path, rotation, max_files, filename_prefix.as_deref())?;
            let (nb, g) = tracing_appender::non_blocking(fa);
            (nb, g, formatter)
        }
    };

    let layer1 = fmt::layer()
        .with_writer(nb1)
        .with_target(config.include_target)
        .with_file(config.include_file)
        .with_line_number(config.include_line_number)
        .with_timer(fmt::time::LocalTime::rfc_3339())
        .with_ansi(true);

    let registry = tracing_subscriber::registry().with(env_filter);

    // Apply formatter
    match f1 {
        Formatter::Compact => {
            let l1 = layer1.compact();
            if let Some(rl) = rate_limit_cfg {
                let limiter = Arc::new(RateLimiter::new(rl.max_per_second));
                registry
                    .with(RateLimitLayer { inner: l1, limiter })
                    .try_init()
                    .map_err(|e| XlogError::InitFailed(e.to_string()))?;
            } else {
                registry
                    .with(l1)
                    .try_init()
                    .map_err(|e| XlogError::InitFailed(e.to_string()))?;
            }
        }
        Formatter::Pretty => {
            let l1 = layer1.pretty();
            if let Some(rl) = rate_limit_cfg {
                let limiter = Arc::new(RateLimiter::new(rl.max_per_second));
                registry
                    .with(RateLimitLayer { inner: l1, limiter })
                    .try_init()
                    .map_err(|e| XlogError::InitFailed(e.to_string()))?;
            } else {
                registry
                    .with(l1)
                    .try_init()
                    .map_err(|e| XlogError::InitFailed(e.to_string()))?;
            }
        }
        Formatter::Json => {
            let l1 = layer1.json();
            if let Some(rl) = rate_limit_cfg {
                let limiter = Arc::new(RateLimiter::new(rl.max_per_second));
                registry
                    .with(RateLimitLayer { inner: l1, limiter })
                    .try_init()
                    .map_err(|e| XlogError::InitFailed(e.to_string()))?;
            } else {
                registry
                    .with(l1)
                    .try_init()
                    .map_err(|e| XlogError::InitFailed(e.to_string()))?;
            }
        }
    }

    Ok(create_multi_guard(vec![g1]))
}

/// Initialize with two outputs
fn initialize_dual_multi(
    outputs: Vec<OutputWithFormatter>,
    env_filter: EnvFilter,
    config: &Config,
    rate_limit_cfg: Option<RateLimit>,
) -> Result<WorkerGuard> {
    use tracing_subscriber::layer::SubscriberExt;

    let mut iter = outputs.into_iter();
    let output1 = iter.next().unwrap();
    let output2 = iter.next().unwrap();

    // Create writers for both outputs
    let (nb1, g1) = match &output1 {
        OutputWithFormatter::Stdout { .. } => tracing_appender::non_blocking(std::io::stdout()),
        OutputWithFormatter::Stderr { .. } => tracing_appender::non_blocking(std::io::stderr()),
        OutputWithFormatter::File {
            path,
            rotation,
            max_files,
            filename_prefix,
            ..
        } => {
            let fa = build_file_appender(path, *rotation, *max_files, filename_prefix.as_deref())?;
            tracing_appender::non_blocking(fa)
        }
    };

    let (nb2, g2) = match &output2 {
        OutputWithFormatter::Stdout { .. } => tracing_appender::non_blocking(std::io::stdout()),
        OutputWithFormatter::Stderr { .. } => tracing_appender::non_blocking(std::io::stderr()),
        OutputWithFormatter::File {
            path,
            rotation,
            max_files,
            filename_prefix,
            ..
        } => {
            let fa = build_file_appender(path, *rotation, *max_files, filename_prefix.as_deref())?;
            tracing_appender::non_blocking(fa)
        }
    };

    // Create layers
    let layer1 = fmt::layer()
        .with_writer(nb1)
        .with_target(config.include_target)
        .with_file(config.include_file)
        .with_line_number(config.include_line_number)
        .with_timer(fmt::time::LocalTime::rfc_3339())
        .with_ansi(true)
        .compact(); // Use compact for simplicity

    let layer2 = fmt::layer()
        .with_writer(nb2)
        .with_target(config.include_target)
        .with_file(config.include_file)
        .with_line_number(config.include_line_number)
        .with_timer(fmt::time::LocalTime::rfc_3339())
        .with_ansi(true)
        .compact(); // Use compact for simplicity

    let registry = tracing_subscriber::registry().with(env_filter);

    // Always wrap in RateLimitLayer for consistent types
    let limiter = rate_limit_cfg
        .map(|rl| Arc::new(RateLimiter::new(rl.max_per_second)))
        .unwrap_or_else(|| Arc::new(RateLimiter::new(u32::MAX))); // Effectively unlimited

    registry
        .with(RateLimitLayer {
            inner: layer1,
            limiter: limiter.clone(),
        })
        .with(RateLimitLayer {
            inner: layer2,
            limiter,
        })
        .try_init()
        .map_err(|e| XlogError::InitFailed(e.to_string()))?;

    Ok(create_multi_guard(vec![g1, g2]))
}

/// Initialize with three outputs
fn initialize_triple_multi(
    outputs: Vec<OutputWithFormatter>,
    env_filter: EnvFilter,
    config: &Config,
    rate_limit_cfg: Option<RateLimit>,
) -> Result<WorkerGuard> {
    use tracing_subscriber::layer::SubscriberExt;

    let mut iter = outputs.into_iter();
    let output1 = iter.next().unwrap();
    let output2 = iter.next().unwrap();
    let output3 = iter.next().unwrap();

    // Create writers for all three outputs
    let (nb1, g1) = match &output1 {
        OutputWithFormatter::Stdout { .. } => tracing_appender::non_blocking(std::io::stdout()),
        OutputWithFormatter::Stderr { .. } => tracing_appender::non_blocking(std::io::stderr()),
        OutputWithFormatter::File {
            path,
            rotation,
            max_files,
            filename_prefix,
            ..
        } => {
            let fa = build_file_appender(path, *rotation, *max_files, filename_prefix.as_deref())?;
            tracing_appender::non_blocking(fa)
        }
    };

    let (nb2, g2) = match &output2 {
        OutputWithFormatter::Stdout { .. } => tracing_appender::non_blocking(std::io::stdout()),
        OutputWithFormatter::Stderr { .. } => tracing_appender::non_blocking(std::io::stderr()),
        OutputWithFormatter::File {
            path,
            rotation,
            max_files,
            filename_prefix,
            ..
        } => {
            let fa = build_file_appender(path, *rotation, *max_files, filename_prefix.as_deref())?;
            tracing_appender::non_blocking(fa)
        }
    };

    let (nb3, g3) = match &output3 {
        OutputWithFormatter::Stdout { .. } => tracing_appender::non_blocking(std::io::stdout()),
        OutputWithFormatter::Stderr { .. } => tracing_appender::non_blocking(std::io::stderr()),
        OutputWithFormatter::File {
            path,
            rotation,
            max_files,
            filename_prefix,
            ..
        } => {
            let fa = build_file_appender(path, *rotation, *max_files, filename_prefix.as_deref())?;
            tracing_appender::non_blocking(fa)
        }
    };

    // Create layers
    let layer1 = fmt::layer()
        .with_writer(nb1)
        .with_target(config.include_target)
        .with_file(config.include_file)
        .with_line_number(config.include_line_number)
        .with_timer(fmt::time::LocalTime::rfc_3339())
        .with_ansi(true)
        .compact();

    let layer2 = fmt::layer()
        .with_writer(nb2)
        .with_target(config.include_target)
        .with_file(config.include_file)
        .with_line_number(config.include_line_number)
        .with_timer(fmt::time::LocalTime::rfc_3339())
        .with_ansi(true)
        .compact();

    let layer3 = fmt::layer()
        .with_writer(nb3)
        .with_target(config.include_target)
        .with_file(config.include_file)
        .with_line_number(config.include_line_number)
        .with_timer(fmt::time::LocalTime::rfc_3339())
        .with_ansi(true)
        .compact();

    let registry = tracing_subscriber::registry().with(env_filter);

    // Always wrap in RateLimitLayer for consistent types
    let limiter = rate_limit_cfg
        .map(|rl| Arc::new(RateLimiter::new(rl.max_per_second)))
        .unwrap_or_else(|| Arc::new(RateLimiter::new(u32::MAX))); // Effectively unlimited

    registry
        .with(RateLimitLayer {
            inner: layer1,
            limiter: limiter.clone(),
        })
        .with(RateLimitLayer {
            inner: layer2,
            limiter: limiter.clone(),
        })
        .with(RateLimitLayer {
            inner: layer3,
            limiter,
        })
        .try_init()
        .map_err(|e| XlogError::InitFailed(e.to_string()))?;

    Ok(create_multi_guard(vec![g1, g2, g3]))
}

/// Create a multi-guard that wraps multiple WorkerGuards
fn create_multi_guard(mut guards: Vec<WorkerGuard>) -> WorkerGuard {
    // Since WorkerGuard doesn't support composition natively,
    // we use a simple approach: return the first guard and leak the rest
    // This ensures all background workers stay alive for the program's lifetime

    if guards.is_empty() {
        panic!("create_multi_guard called with empty guards vector");
    }

    // Take the first guard to return
    let primary_guard = guards.remove(0);

    // Leak the remaining guards to keep them alive
    // They will be cleaned up when the program exits
    if !guards.is_empty() {
        let boxed = Box::new(MultiGuardInner { _guards: guards });
        Box::leak(boxed);
    }

    primary_guard
}

/// Initialize single output logging (refactored from do_initialize)
fn initialize_single_output(
    output: Output,
    formatter: Formatter,
    env_filter: EnvFilter,
    config: &Config,
    rate_limit_cfg: Option<RateLimit>,
) -> Result<WorkerGuard> {
    use tracing_subscriber::layer::SubscriberExt;

    // Determine if ANSI colors should be enabled
    let is_terminal_output = matches!(output, Output::Stdout | Output::Stderr);

    // Build output writer
    let (non_blocking, guard) = match output {
        Output::Stdout => tracing_appender::non_blocking(std::io::stdout()),
        Output::Stderr => tracing_appender::non_blocking(std::io::stderr()),
        Output::File {
            path,
            rotation,
            max_files,
            filename_prefix,
        } => {
            let file_appender =
                build_file_appender(path, rotation, max_files, filename_prefix.as_deref())?;
            tracing_appender::non_blocking(file_appender)
        }
        Output::Multi(_) => {
            return Err(XlogError::InvalidConfig(
                "Multi output should be handled by initialize_multi_output".to_string(),
            ));
        }
    };

    // Create the base layer
    let base_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_target(config.include_target)
        .with_file(config.include_file)
        .with_line_number(config.include_line_number)
        .with_timer(fmt::time::LocalTime::rfc_3339())
        .with_ansi(is_terminal_output);

    let registry = tracing_subscriber::registry().with(env_filter);

    // Apply formatting and rate limiting
    match formatter {
        Formatter::Compact => {
            let fmt_layer = base_layer.compact();
            if let Some(rate_limit) = rate_limit_cfg {
                let limiter = Arc::new(RateLimiter::new(rate_limit.max_per_second));
                registry
                    .with(RateLimitLayer {
                        inner: fmt_layer,
                        limiter,
                    })
                    .try_init()
                    .map_err(|e| XlogError::InitFailed(e.to_string()))?;
            } else {
                registry
                    .with(fmt_layer)
                    .try_init()
                    .map_err(|e| XlogError::InitFailed(e.to_string()))?;
            }
        }
        Formatter::Pretty => {
            let fmt_layer = base_layer.pretty();
            if let Some(rate_limit) = rate_limit_cfg {
                let limiter = Arc::new(RateLimiter::new(rate_limit.max_per_second));
                registry
                    .with(RateLimitLayer {
                        inner: fmt_layer,
                        limiter,
                    })
                    .try_init()
                    .map_err(|e| XlogError::InitFailed(e.to_string()))?;
            } else {
                registry
                    .with(fmt_layer)
                    .try_init()
                    .map_err(|e| XlogError::InitFailed(e.to_string()))?;
            }
        }
        Formatter::Json => {
            let fmt_layer = base_layer.json();
            if let Some(rate_limit) = rate_limit_cfg {
                let limiter = Arc::new(RateLimiter::new(rate_limit.max_per_second));
                registry
                    .with(RateLimitLayer {
                        inner: fmt_layer,
                        limiter,
                    })
                    .try_init()
                    .map_err(|e| XlogError::InitFailed(e.to_string()))?;
            } else {
                registry
                    .with(fmt_layer)
                    .try_init()
                    .map_err(|e| XlogError::InitFailed(e.to_string()))?;
            }
        }
    }

    Ok(guard)
}
