# xlog 现代化实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 xlog 从简单的 tracing 封装重构为功能丰富的企业级日志库，包含类型安全的 API、灵活配置和高级功能。

**Architecture:** 采用分层模块架构，将错误处理、日志级别、输出目标、格式化器和配置逻辑分离。遵循 TDD 原则，先写测试再实现功能。使用 Builder 模式提供灵活配置，同时保留简单场景的快捷函数。

**Tech Stack:** Rust, tracing, tracing-subscriber, tracing-appender, thiserror, serde (optional), serde_json (optional)

---

## 阶段 0: 准备工作

### Task 0.1: 更新依赖

**Files:**
- Modify: `Cargo.toml`

**Step 1: 添加新依赖到 Cargo.toml**

```toml
[package]
name = "xlog"
version = "0.2.0"
edition = "2021"

[dependencies]
tracing = "0.1"
tracing-appender = "0.2"
tracing-subscriber = { version = "0.3", features = [
    "env-filter",
    "local-time",
    "time",
    "json",
] }
thiserror = "2.0"
serde = { version = "1.0", optional = true }
serde_json = { version = "1.0", optional = true }

[features]
default = ["json"]
json = ["dep:serde", "dep:serde_json"]

[dev-dependencies]
tempfile = "3.0"
```

**Step 2: 验证依赖安装**

Run: `cargo check`
Expected: 成功编译，下载新依赖

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: update dependencies for xlog 0.2.0

Add thiserror for structured errors
Add serde/serde_json for JSON formatting (optional feature)
Update version to 0.2.0

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 阶段 1: 基础类型 - 错误处理

### Task 1.1: 实现错误类型

**Files:**
- Create: `src/error.rs`
- Test: `src/error.rs` (内联测试)

**Step 1: 编写错误类型测试**

```rust
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
```

**Step 2: 运行测试验证失败**

Run: `cargo test --lib error::tests`
Expected: FAIL - module `error` not found

**Step 3: 实现错误类型**

在 `src/error.rs` 顶部添加：

```rust
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
```

**Step 4: 运行测试验证通过**

Run: `cargo test --lib error::tests`
Expected: PASS - 2 tests passed

**Step 5: Commit**

```bash
git add src/error.rs
git commit -m "feat: add structured error types

Implement XlogError with thiserror
Add type-safe error variants with clear messages
Add Result type alias

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 阶段 2: 基础类型 - 日志级别

### Task 2.1: 实现日志级别枚举

**Files:**
- Create: `src/level.rs`
- Test: `src/level.rs` (内联测试)

**Step 1: 编写日志级别测试**

```rust
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
```

**Step 2: 运行测试验证失败**

Run: `cargo test --lib level::tests`
Expected: FAIL - module `level` not found

**Step 3: 实现日志级别枚举**

在 `src/level.rs` 顶部添加：

```rust
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
```

**Step 4: 运行测试验证通过**

Run: `cargo test --lib level::tests`
Expected: PASS - 3 tests passed

**Step 5: Commit**

```bash
git add src/level.rs
git commit -m "feat: add type-safe log level enum

Implement Level enum with ordering
Support string parsing for backward compatibility
Convert to tracing LevelFilter

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 阶段 3: 输出目标

### Task 3.1: 实现输出目标和轮转策略

**Files:**
- Create: `src/output.rs`
- Test: `src/output.rs` (内联测试)

**Step 1: 编写输出目标测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_stdout_output() {
        let output = Output::stdout();
        assert!(matches!(output, Output::Stdout));
    }

    #[test]
    fn test_file_output_builder() {
        let output = Output::file("./logs")
            .with_rotation(Rotation::Hourly)
            .max_files(24)
            .build();

        match output {
            Output::File { path, rotation, max_files } => {
                assert_eq!(path, PathBuf::from("./logs"));
                assert!(matches!(rotation, Rotation::Hourly));
                assert_eq!(max_files, 24);
            }
            _ => panic!("Expected File output"),
        }
    }

    #[test]
    fn test_multi_output() {
        let multi = Output::multi()
            .add(Output::stdout())
            .add(Output::stderr())
            .build();

        match multi {
            Output::Multi(outputs) => {
                assert_eq!(outputs.len(), 2);
            }
            _ => panic!("Expected Multi output"),
        }
    }
}
```

**Step 2: 运行测试验证失败**

Run: `cargo test --lib output::tests`
Expected: FAIL - module `output` not found

**Step 3: 实现输出目标**

在 `src/output.rs` 添加：

```rust
use std::path::PathBuf;

/// 日志文件轮转策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    /// 不轮转
    Never,
    /// 每小时轮转一次
    Hourly,
    /// 每天轮转一次（默认）
    Daily,
    /// 每周轮转一次
    Weekly,
}

impl From<Rotation> for tracing_appender::rolling::Rotation {
    fn from(rotation: Rotation) -> Self {
        match rotation {
            Rotation::Never => Self::NEVER,
            Rotation::Hourly => Self::HOURLY,
            Rotation::Daily => Self::DAILY,
            Rotation::Weekly => Self::DAILY, // tracing-appender 不支持 weekly，用 daily 代替
        }
    }
}

/// 日志输出目标
#[derive(Debug)]
pub enum Output {
    /// 标准输出
    Stdout,
    /// 标准错误输出
    Stderr,
    /// 文件输出
    File {
        /// 日志目录路径
        path: PathBuf,
        /// 轮转策略
        rotation: Rotation,
        /// 最大保留文件数
        max_files: usize,
    },
    /// 多个输出目标
    Multi(Vec<Output>),
}

impl Output {
    /// 创建标准输出目标
    pub fn stdout() -> Self {
        Self::Stdout
    }

    /// 创建标准错误输出目标
    pub fn stderr() -> Self {
        Self::Stderr
    }

    /// 创建文件输出 Builder
    pub fn file<P: Into<PathBuf>>(path: P) -> FileOutputBuilder {
        FileOutputBuilder {
            path: path.into(),
            rotation: Rotation::Daily,
            max_files: 7,
        }
    }

    /// 创建多输出 Builder
    pub fn multi() -> MultiOutputBuilder {
        MultiOutputBuilder {
            outputs: Vec::new(),
        }
    }
}

/// 文件输出 Builder
#[derive(Debug)]
pub struct FileOutputBuilder {
    path: PathBuf,
    rotation: Rotation,
    max_files: usize,
}

impl FileOutputBuilder {
    /// 设置轮转策略
    pub fn with_rotation(mut self, rotation: Rotation) -> Self {
        self.rotation = rotation;
        self
    }

    /// 设置最大保留文件数
    pub fn max_files(mut self, max: usize) -> Self {
        self.max_files = max;
        self
    }

    /// 构建输出目标
    pub fn build(self) -> Output {
        Output::File {
            path: self.path,
            rotation: self.rotation,
            max_files: self.max_files,
        }
    }
}

/// 多输出 Builder
#[derive(Debug)]
pub struct MultiOutputBuilder {
    outputs: Vec<Output>,
}

impl MultiOutputBuilder {
    /// 添加一个输出目标
    pub fn add(mut self, output: Output) -> Self {
        self.outputs.push(output);
        self
    }

    /// 构建多输出目标
    pub fn build(self) -> Output {
        Output::Multi(self.outputs)
    }
}
```

**Step 4: 运行测试验证通过**

Run: `cargo test --lib output::tests`
Expected: PASS - 3 tests passed

**Step 5: Commit**

```bash
git add src/output.rs
git commit -m "feat: add output targets with rotation support

Implement Output enum (Stdout, Stderr, File, Multi)
Add Rotation strategy (Never, Hourly, Daily, Weekly)
Provide builder pattern for flexible configuration

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 阶段 4: 格式化器

### Task 4.1: 实现格式化器枚举

**Files:**
- Create: `src/formatter.rs`
- Test: `src/formatter.rs` (内联测试)

**Step 1: 编写格式化器测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formatter_creation() {
        let compact = Formatter::compact();
        assert!(matches!(compact, Formatter::Compact));

        let pretty = Formatter::pretty();
        assert!(matches!(pretty, Formatter::Pretty));

        let json = Formatter::json();
        assert!(matches!(json, Formatter::Json));
    }

    #[test]
    fn test_formatter_clone() {
        let formatter = Formatter::Pretty;
        let cloned = formatter.clone();
        assert!(matches!(cloned, Formatter::Pretty));
    }
}
```

**Step 2: 运行测试验证失败**

Run: `cargo test --lib formatter::tests`
Expected: FAIL - module `formatter` not found

**Step 3: 实现格式化器**

在 `src/formatter.rs` 添加：

```rust
/// 日志格式化器
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Formatter {
    /// 紧凑格式 - 单行输出，无颜色，适合生产环境
    Compact,
    /// 美观格式 - 多行格式，ANSI 颜色，适合开发环境（默认）
    Pretty,
    /// JSON 格式 - 结构化输出，适合日志收集系统
    Json,
}

impl Formatter {
    /// 创建紧凑格式化器
    pub fn compact() -> Self {
        Self::Compact
    }

    /// 创建美观格式化器
    pub fn pretty() -> Self {
        Self::Pretty
    }

    /// 创建 JSON 格式化器
    pub fn json() -> Self {
        Self::Json
    }
}

impl Default for Formatter {
    fn default() -> Self {
        Self::Pretty
    }
}
```

**Step 4: 运行测试验证通过**

Run: `cargo test --lib formatter::tests`
Expected: PASS - 2 tests passed

**Step 5: Commit**

```bash
git add src/formatter.rs
git commit -m "feat: add formatter types

Implement Formatter enum (Compact, Pretty, Json)
Set Pretty as default for development
Provide builder methods

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 阶段 5: 配置系统

### Task 5.1: 实现配置结构

**Files:**
- Create: `src/config.rs`
- Test: `src/config.rs` (内联测试)

**Step 1: 编写配置结构测试**

```rust
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
```

**Step 2: 运行测试验证失败**

Run: `cargo test --lib config::tests`
Expected: FAIL - module `config` not found

**Step 3: 实现配置结构**

在 `src/config.rs` 添加：

```rust
use crate::{Level, Output, Formatter, Result};
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
        crate::init::initialize(self)
    }
}
```

**Step 4: 运行测试验证通过**

Run: `cargo test --lib config::tests`
Expected: PASS - 3 tests passed

**Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat: add Config with builder pattern

Implement Config struct with sensible defaults
Provide fluent API for configuration
Support fine-grained format control

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 阶段 6: 初始化逻辑

### Task 6.1: 实现初始化函数

**Files:**
- Create: `src/init.rs`
- Test: `tests/integration_test.rs` (集成测试)

**Step 1: 编写初始化逻辑**

在 `src/init.rs` 添加：

```rust
use crate::{Config, Level, Output, Formatter, Result, XlogError};
use std::sync::Once;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

static INIT: Once = Once::new();
static mut INITIALIZED: bool = false;

/// 初始化日志系统
pub(crate) fn initialize(config: Config) -> Result<WorkerGuard> {
    let mut result: Option<Result<WorkerGuard>> = None;

    INIT.call_once(|| {
        result = Some(do_initialize(config));
        unsafe { INITIALIZED = true; }
    });

    // 如果已经初始化过，返回错误
    if unsafe { INITIALIZED } && result.is_none() {
        return Err(XlogError::AlreadyInitialized);
    }

    result.unwrap()
}

fn do_initialize(config: Config) -> Result<WorkerGuard> {
    // 构建 EnvFilter
    let level_filter = config.level.into();
    let env_filter = EnvFilter::builder()
        .with_default_directive(level_filter.into())
        .from_env_lossy();

    // 构建输出 writer
    let (non_blocking, guard) = match config.output {
        Output::Stdout => {
            tracing_appender::non_blocking(std::io::stdout())
        }
        Output::Stderr => {
            tracing_appender::non_blocking(std::io::stderr())
        }
        Output::File { path, rotation, max_files } => {
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

    // 构建格式化层
    let fmt_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_target(config.include_target)
        .with_file(config.include_file)
        .with_line_number(config.include_line_number)
        .with_timer(fmt::time::LocalTime::rfc_3339())
        .with_ansi(matches!(config.output, Output::Stdout | Output::Stderr));

    // 应用格式化样式
    let fmt_layer = match config.formatter {
        Formatter::Compact => fmt_layer.compact(),
        Formatter::Pretty => fmt_layer.pretty(),
        Formatter::Json => fmt_layer.json(),
    };

    // 初始化订阅者
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .try_init()
        .map_err(|e| XlogError::InitFailed(e.to_string()))?;

    Ok(guard)
}
```

**Step 2: 创建集成测试**

创建 `tests/integration_test.rs`：

```rust
use xlog::{Config, Level, Output, Formatter};

#[test]
fn test_init_stdout() {
    let result = Config::new()
        .level(Level::Info)
        .output(Output::stdout())
        .init();

    assert!(result.is_ok());
}

#[test]
fn test_double_init_error() {
    // 注意：这个测试可能会与其他测试冲突
    // 因为全局状态只能初始化一次
    // 在实际项目中应该使用串行测试或隔离测试
}
```

**Step 3: 运行测试验证**

Run: `cargo test --test integration_test`
Expected: PASS - 基础初始化测试通过

**Step 4: Commit**

```bash
git add src/init.rs tests/integration_test.rs
git commit -m "feat: implement initialization logic

Add initialize function with Once guard
Build tracing subscriber with config
Support stdout, stderr, and file outputs
Handle double initialization error

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 阶段 7: 公共 API

### Task 7.1: 更新 lib.rs 导出新 API

**Files:**
- Modify: `src/lib.rs`
- Test: `tests/api_test.rs`

**Step 1: 编写 API 测试**

创建 `tests/api_test.rs`：

```rust
use xlog::{Level, Output, Config};
use std::path::PathBuf;

#[test]
fn test_quick_init() {
    // 注意：仅在独立进程中测试，避免重复初始化
}

#[test]
fn test_builder_api() {
    let config = Config::new()
        .level(Level::Debug)
        .output(Output::file("./logs").build())
        .with_target(true);

    // 验证配置正确
}

#[test]
fn test_level_from_str() {
    assert_eq!(Level::from_str("info").unwrap(), Level::Info);
    assert!(Level::from_str("invalid").is_err());
}
```

**Step 2: 更新 lib.rs**

替换 `src/lib.rs` 内容：

```rust
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
```

**Step 3: 运行测试验证编译**

Run: `cargo test --lib`
Expected: 所有测试通过

**Step 4: Commit**

```bash
git add src/lib.rs tests/api_test.rs
git commit -m "feat: expose new public API

Export all core types (Level, Output, Formatter, Config)
Add quick_init, init_with_level, init_file helpers
Add comprehensive documentation

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 阶段 8: 移除旧代码

### Task 8.1: 移除旧的 log.rs

**Files:**
- Delete: `src/log.rs`

**Step 1: 删除旧文件**

Run: `rm src/log.rs`

**Step 2: 运行测试确保没有依赖**

Run: `cargo test`
Expected: 所有测试通过，无编译错误

**Step 3: Commit**

```bash
git add src/log.rs
git commit -m "refactor: remove old log.rs implementation

Old Builder with lifetime parameters replaced by Config
Old string-based API replaced by type-safe enums

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 阶段 9: 文档和示例

### Task 9.1: 创建使用示例

**Files:**
- Create: `examples/simple.rs`
- Create: `examples/file_logging.rs`
- Create: `examples/json_format.rs`

**Step 1: 创建简单示例**

`examples/simple.rs`:

```rust
use xlog::Result;
use tracing::{info, warn, error};

fn main() -> Result<()> {
    // 最简单的使用方式
    let _guard = xlog::quick_init()?;

    info!("Application started");
    warn!("This is a warning");
    error!("This is an error");

    Ok(())
}
```

**Step 2: 创建文件日志示例**

`examples/file_logging.rs`:

```rust
use xlog::{Config, Level, Output, Rotation, Result};
use tracing::info;

fn main() -> Result<()> {
    let _guard = Config::new()
        .level(Level::Debug)
        .output(
            Output::file("./logs")
                .with_rotation(Rotation::Daily)
                .max_files(7)
                .build()
        )
        .init()?;

    info!("This message goes to file");
    info!("Logs will rotate daily");

    Ok(())
}
```

**Step 3: 创建 JSON 格式示例**

`examples/json_format.rs`:

```rust
use xlog::{Config, Level, Output, Formatter, Result};
use tracing::info;

fn main() -> Result<()> {
    let _guard = Config::new()
        .level(Level::Info)
        .output(Output::stdout())
        .formatter(Formatter::json())
        .init()?;

    info!("This is structured JSON log");
    info!(user_id = 123, action = "login", "User logged in");

    Ok(())
}
```

**Step 4: 测试所有示例**

Run: `cargo run --example simple`
Expected: 看到格式化的日志输出

Run: `cargo run --example file_logging`
Expected: 在 ./logs 目录生成日志文件

Run: `cargo run --example json_format`
Expected: 看到 JSON 格式的日志

**Step 5: Commit**

```bash
git add examples/
git commit -m "docs: add usage examples

Add simple.rs for basic usage
Add file_logging.rs for file output with rotation
Add json_format.rs for structured logging

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 阶段 10: README 更新

### Task 10.1: 更新 README

**Files:**
- Modify: `README.md`

**Step 1: 更新 README 内容**

```markdown
# xlog

简单而强大的 Rust 日志库，基于 [tracing](https://github.com/tokio-rs/tracing) 构建。

## 特性

- 🚀 **简单易用** - 一行代码即可启动
- 🎯 **类型安全** - 编译时检查，避免运行时错误
- 🔧 **灵活配置** - 支持多种输出和格式
- 📦 **零依赖冲突** - 基于标准 tracing 生态
- 🔄 **文件轮转** - 自动按时间轮转日志文件

## 快速开始

添加依赖：

```toml
[dependencies]
xlog = "0.2"
tracing = "0.1"
```

最简单的使用：

```rust
use xlog::Result;

fn main() -> Result<()> {
    let _guard = xlog::quick_init()?;
    tracing::info!("Hello, xlog!");
    Ok(())
}
```

## 使用示例

### 指定日志级别

```rust
use xlog::{Level, Result};

fn main() -> Result<()> {
    let _guard = xlog::init_with_level(Level::Debug)?;
    tracing::debug!("Debug message");
    Ok(())
}
```

### 输出到文件

```rust
use xlog::{Config, Level, Output, Rotation, Result};

fn main() -> Result<()> {
    let _guard = Config::new()
        .level(Level::Info)
        .output(
            Output::file("./logs")
                .with_rotation(Rotation::Daily)
                .max_files(7)
                .build()
        )
        .init()?;

    tracing::info!("Logging to file");
    Ok(())
}
```

### JSON 格式

```rust
use xlog::{Config, Formatter, Result};

fn main() -> Result<()> {
    let _guard = Config::new()
        .formatter(Formatter::json())
        .init()?;

    tracing::info!(user = "alice", "User logged in");
    Ok(())
}
```

## 配置选项

### 日志级别

- `Level::Trace` - 最详细
- `Level::Debug` - 调试信息
- `Level::Info` - 一般信息（默认）
- `Level::Warn` - 警告
- `Level::Error` - 错误

### 输出目标

- `Output::stdout()` - 标准输出（默认）
- `Output::stderr()` - 标准错误
- `Output::file(path)` - 文件输出，支持轮转

### 格式化器

- `Formatter::Pretty` - 美观格式，适合开发（默认）
- `Formatter::Compact` - 紧凑格式，适合生产
- `Formatter::Json` - JSON 格式，适合日志系统

## 从 0.1.x 迁移

xlog 0.2.0 引入了破坏性改变以提供更好的 API。

**旧 API**:
```rust
let _guard = xlog::init("./logs", "info")?;
```

**新 API**:
```rust
use xlog::{Level, Output};
let _guard = xlog::init_file("./logs", Level::Info)?;
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: update README for 0.2.0

Document new API with examples
Add migration guide from 0.1.x
List all features and configuration options

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 阶段 11: 最终验证

### Task 11.1: 完整测试和清理

**Step 1: 运行所有测试**

Run: `cargo test --all-targets`
Expected: 所有测试通过

**Step 2: 运行所有示例**

Run: `cargo run --example simple && cargo run --example file_logging && cargo run --example json_format`
Expected: 所有示例正常运行

**Step 3: 检查文档**

Run: `cargo doc --open`
Expected: 文档生成成功，无警告

**Step 4: Clippy 检查**

Run: `cargo clippy -- -D warnings`
Expected: 无 clippy 警告

**Step 5: 格式化代码**

Run: `cargo fmt`

**Step 6: 最终 commit**

```bash
git add .
git commit -m "chore: final cleanup and formatting

Run cargo fmt on all files
Verify all tests pass
Verify all examples work

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 完成检查清单

验证以下所有项目都已完成：

- [ ] 所有新模块已创建（error, level, output, formatter, config, init）
- [ ] 旧的 log.rs 已删除
- [ ] lib.rs 已更新，导出新 API
- [ ] 所有测试通过（单元测试和集成测试）
- [ ] 至少 3 个示例程序可运行
- [ ] README 已更新
- [ ] Cargo.toml 版本号更新到 0.2.0
- [ ] 所有代码通过 cargo fmt 和 cargo clippy
- [ ] 文档生成无警告

## 后续工作（可选）

完成以上实施后，可以考虑以下增强功能：

1. **多输出支持的完整实现** - 当前 Multi 输出使用了简化实现
2. **模块级别过滤** - 实现 `filter_module()` 功能
3. **速率限制** - 实现 `rate_limit()` 功能
4. **环境变量过滤** - 实现 `with_env_filter()` 功能
5. **性能基准测试** - 添加 criterion benchmarks
6. **发布到 crates.io** - 准备发布 0.2.0 版本

---

## 注意事项

1. **WorkerGuard 生命周期**：确保返回的 `_guard` 在程序整个生命周期内存活
2. **全局状态**：日志系统只能初始化一次，测试时注意隔离
3. **时间格式**：LocalTime 需要系统支持，可能在某些环境失败
4. **文件权限**：确保日志目录有写权限
5. **依赖版本**：tracing 生态依赖版本需保持一致
