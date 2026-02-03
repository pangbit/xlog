//! xlog - 简单而强大的 Rust 日志库
//!
//! xlog 是对 tracing 的友好封装，提供简单易用的 API 和强大的配置能力。
//!
//! # 快速开始
//!
//! ```no_run
//! use xlog::Result;
//!
//! fn main() -> Result<()> {
//!     let _guard = xlog::quick_init()?;
//!     tracing::info!("Application started");
//!     Ok(())
//! }
//! ```
//!
//! # 完整配置
//!
//! ```no_run
//! use xlog::{Config, Level, Output, Formatter, Result};
//!
//! fn main() -> Result<()> {
//!     let _guard = Config::new()
//!         .level(Level::Info)
//!         .output(Output::file("./logs").build())
//!         .formatter(Formatter::json())
//!         .init()?;
//!
//!     tracing::info!("Logging to file");
//!     Ok(())
//! }
//! ```

mod error;
mod level;
mod output;
mod formatter;
mod config;
mod init;

// 导出公共类型
pub use error::{XlogError, Result};
pub use level::Level;
pub use output::{Output, Rotation, FileOutputBuilder, MultiOutputBuilder};
pub use formatter::Formatter;
pub use config::Config;

use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;

/// 快速初始化日志系统（使用默认配置）
///
/// 默认配置：
/// - 级别: Info
/// - 输出: Stdout
/// - 格式: Pretty
///
/// # 示例
///
/// ```no_run
/// # use xlog::Result;
/// fn main() -> Result<()> {
///     let _guard = xlog::quick_init()?;
///     tracing::info!("Hello, xlog!");
///     Ok(())
/// }
/// ```
pub fn quick_init() -> Result<WorkerGuard> {
    Config::default().init()
}

/// 使用指定级别初始化日志系统
///
/// # 示例
///
/// ```no_run
/// # use xlog::{Level, Result};
/// fn main() -> Result<()> {
///     let _guard = xlog::init_with_level(Level::Debug)?;
///     tracing::debug!("Debug message");
///     Ok(())
/// }
/// ```
pub fn init_with_level(level: Level) -> Result<WorkerGuard> {
    Config::new().level(level).init()
}

/// 初始化文件日志
///
/// # 示例
///
/// ```no_run
/// # use xlog::{Level, Result};
/// fn main() -> Result<()> {
///     let _guard = xlog::init_file("./logs", Level::Info)?;
///     tracing::info!("Logging to file");
///     Ok(())
/// }
/// ```
pub fn init_file<P: Into<PathBuf>>(path: P, level: Level) -> Result<WorkerGuard> {
    Config::new()
        .level(level)
        .output(Output::file(path).build())
        .init()
}

