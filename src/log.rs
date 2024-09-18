#![allow(dead_code)]

use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{filter::LevelFilter, fmt, EnvFilter};

pub fn init<P: AsRef<Path>>(
    logs: P,
    level: &str,
) -> Result<WorkerGuard, Box<dyn std::error::Error>> {
    //log level
    let level = match level {
        "debug" => LevelFilter::DEBUG,
        "info" => LevelFilter::INFO,
        "warn" => LevelFilter::WARN,
        "error" => LevelFilter::ERROR,
        _ => LevelFilter::DEBUG,
    };
    let flt_layer = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy();

    //log dest
    let use_stdout = logs.as_ref().to_string_lossy().to_ascii_lowercase() == "stdout";
    let non_blocking;
    let guard;

    if use_stdout {
        (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());
    } else {
        let file_appender = RollingFileAppender::builder()
            .filename_prefix("main.log")
            .rotation(Rotation::DAILY)
            .max_log_files(15)
            .build(logs)?;

        (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    }

    let mut out_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(false);

    if use_stdout {
        out_layer = out_layer.with_ansi(true);
    }

    tracing_subscriber::registry()
        .with(flt_layer)
        .with(out_layer)
        .try_init()?;

    //guard用于保证日志缓冲区成功刷新到输出中
    Ok(guard)
}
