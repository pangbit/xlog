#![allow(dead_code)]

use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{filter::LevelFilter, fmt, EnvFilter};

pub fn init(
    log_home: impl AsRef<Path>,
    log_level: &str,
) -> Result<WorkerGuard, Box<dyn std::error::Error>> {
    init_log(log_home, log_level, "main.log", 3)
}

#[derive(Debug)]
pub struct Builder<'a> {
    log_home: &'a Path,
    log_level: &'a str,
    filename_prefix: &'a str,
    max_log_files: usize,
}

pub fn builder() -> Builder<'static> {
    Builder {
        log_home: Path::new("stdout"),
        log_level: "debug",
        filename_prefix: "main.log",
        max_log_files: 3,
    }
}

impl<'a> Builder<'a> {
    pub fn log_home<P>(mut self, log_home: &'a P) -> Self
    where
        P: AsRef<Path> + ?Sized,
    {
        self.log_home = log_home.as_ref();
        self
    }

    pub fn log_level(mut self, log_level: &'a str) -> Self {
        self.log_level = log_level;
        self
    }

    pub fn filename_prefix(mut self, filename_prefix: &'a str) -> Self {
        self.filename_prefix = filename_prefix;
        self
    }

    pub fn max_log_files(mut self, max_log_files: usize) -> Self {
        self.max_log_files = max_log_files;
        self
    }

    pub fn build(self) -> Result<WorkerGuard, Box<dyn std::error::Error>> {
        init_log(
            self.log_home,
            self.log_level,
            self.filename_prefix,
            self.max_log_files,
        )
    }
}

fn init_log(
    log_home: impl AsRef<Path>,
    log_level: &str,
    filename_prefix: &str,
    max_log_files: usize,
) -> Result<WorkerGuard, Box<dyn std::error::Error>> {
    //log level
    let level = match log_level {
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
    let use_stdout = log_home.as_ref().to_string_lossy().to_ascii_lowercase() == "stdout";
    let non_blocking;
    let guard;

    if use_stdout {
        (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());
    } else {
        let file_appender = RollingFileAppender::builder()
            .filename_prefix(filename_prefix)
            .rotation(Rotation::DAILY)
            .max_log_files(max_log_files)
            .build(log_home)?;

        (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    }

    let mut out_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .with_timer(fmt::time::LocalTime::rfc_3339())
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

#[cfg(test)]
mod tests {
    use tracing::info;

    use super::*;

    #[test]
    fn test_localtime() {
        let _guard = init("stdout", "info").unwrap();
        info!("this is a localtime log");
    }

    #[test]
    fn test_builder() {
        let _guard = builder()
            .log_home("stdout")
            .log_level("info")
            .build()
            .unwrap();
        info!("this is a log from builder");
    }
}
