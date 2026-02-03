# xlog 现代化优化设计方案

**日期**: 2026-02-03
**状态**: 已验证
**目标**: 全面优化 xlog，提升 API 人机工程学、运行时性能、功能完整性和代码质量

## 设计目标

将 xlog 从简单的 tracing 封装升级为功能丰富的企业级日志库，同时保持简单场景的易用性。

### 核心改进

1. **消除生命周期复杂性** - Builder 改用拥有数据而非引用
2. **类型安全的 API** - 用枚举替代字符串（日志级别、格式化器等）
3. **精确的错误处理** - 专门的错误类型替代 `Box<dyn Error>`
4. **灵活的配置** - 支持多输出、多格式、细粒度控制
5. **高级功能** - 结构化日志、速率限制、模块级过滤

## 整体架构

### 模块结构

```
src/
├── lib.rs           # 公共 API 导出和快捷函数
├── error.rs         # 统一错误类型定义
├── level.rs         # 日志级别枚举
├── config.rs        # 配置结构体（新版 Builder）
├── formatter.rs     # 格式化器（JSON、Pretty、Compact）
├── output.rs        # 输出目标抽象（File、Stdout、Multi）
└── init.rs          # 初始化逻辑实现
```

### 公共 API 示例

```rust
// 最简单的使用
let _guard = xlog::quick_init()?;

// 完整配置
let _guard = xlog::Config::new()
    .level(Level::Info)
    .output(Output::file("./logs").with_rotation(Rotation::Daily))
    .formatter(Formatter::pretty())
    .init()?;

// 多输出
let _guard = xlog::Config::new()
    .output(Output::multi()
        .add(Output::stdout())
        .add(Output::file("./logs")))
    .init()?;
```

## 核心类型设计

### 错误类型 (error.rs)

使用 `thiserror` 提供清晰、结构化的错误信息：

```rust
#[derive(Debug, thiserror::Error)]
pub enum XlogError {
    #[error("Failed to initialize logger: {0}")]
    InitFailed(String),

    #[error("Logger already initialized")]
    AlreadyInitialized,

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Invalid log level: {0}")]
    InvalidLevel(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

pub type Result<T> = std::result::Result<T, XlogError>;
```

**设计要点**：
- 每个错误变体语义明确
- 自动从 `io::Error` 转换
- 提供友好的错误消息

### 日志级别 (level.rs)

类型安全的日志级别枚举：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}
```

**功能**：
- 实现 `from_str()` 支持字符串转换（向后兼容）
- 实现 `From<Level> for LevelFilter` 与 tracing 无缝集成
- 支持 "warn" 和 "warning" 等常见别名

### 输出目标 (output.rs)

支持多种输出目标：

```rust
#[derive(Debug)]
pub enum Output {
    Stdout,
    Stderr,
    File {
        path: PathBuf,
        rotation: Rotation,
        max_files: usize,
    },
    Multi(Vec<Output>),
}

#[derive(Debug, Clone)]
pub enum Rotation {
    Never,
    Hourly,
    Daily,
    Weekly,
}
```

**Builder 模式**：
- `Output::file("./logs")` 返回 `FileOutputBuilder`
- 链式配置：`.with_rotation(Rotation::Hourly).max_files(24)`
- `Output::multi()` 支持组合多个输出

**设计亮点**：
- `Multi` 变体允许同时输出到控制台和文件
- 文件输出支持自动轮转和文件数量限制
- 所有类型拥有数据，无生命周期参数

### 格式化器 (formatter.rs)

三种内置格式化器：

```rust
#[derive(Debug, Clone)]
pub enum Formatter {
    Compact,    // 紧凑格式，生产环境
    Pretty,     // 美观格式，开发环境
    Json,       // JSON 格式，日志系统集成
}
```

**特性**：
- `Compact`: 单行输出，无颜色，最小化空间
- `Pretty`: 多行格式，ANSI 颜色，易于阅读
- `Json`: 结构化 JSON，便于解析和检索

### 配置结构 (config.rs)

核心配置 API：

```rust
#[derive(Debug)]
pub struct Config {
    level: Level,
    output: Output,
    formatter: Formatter,
    include_file: bool,
    include_line_number: bool,
    include_target: bool,
}

impl Config {
    pub fn new() -> Self { /* 合理的默认值 */ }

    // 基础配置
    pub fn level(self, level: Level) -> Self;
    pub fn output(self, output: Output) -> Self;
    pub fn formatter(self, formatter: Formatter) -> Self;

    // 格式细节
    pub fn with_file(self, enabled: bool) -> Self;
    pub fn with_line_number(self, enabled: bool) -> Self;
    pub fn with_target(self, enabled: bool) -> Self;

    // 高级过滤
    pub fn with_env_filter(self, env_key: &str) -> Self;
    pub fn filter_module(self, module: &str, level: Level) -> Self;
    pub fn rate_limit(self, max_per_second: u32) -> Self;

    // 初始化
    pub fn init(self) -> Result<WorkerGuard>;
}
```

**默认值**：
- 级别: `Level::Info`
- 输出: `Output::Stdout`
- 格式: `Formatter::Pretty`
- 包含文件名和行号，不包含 target

### 快捷函数 (lib.rs)

为常见场景提供便捷函数：

```rust
// 零配置启动
pub fn quick_init() -> Result<WorkerGuard>;

// 仅指定级别
pub fn init_with_level(level: Level) -> Result<WorkerGuard>;

// 文件输出
pub fn init_file<P: Into<PathBuf>>(path: P, level: Level) -> Result<WorkerGuard>;
```

## 高级功能

### 1. 环境变量过滤

```rust
Config::new()
    .with_env_filter("RUST_LOG")
    .init()?;
```

运行时通过 `RUST_LOG=debug` 覆盖默认级别。

### 2. 模块级别过滤

```rust
Config::new()
    .level(Level::Info)
    .filter_module("hyper", Level::Warn)  // hyper 只输出 warn
    .filter_module("tokio", Level::Error) // tokio 只输出 error
    .init()?;
```

降低依赖库的日志噪音。

### 3. 速率限制

```rust
Config::new()
    .rate_limit(1000)  // 每秒最多 1000 条日志
    .init()?;
```

防止日志洪水攻击或无限循环导致的磁盘填满。

### 4. 多输出组合

```rust
let multi = Output::multi()
    .add(Output::stdout().with_formatter(Formatter::Pretty))
    .add(Output::file("./logs").with_formatter(Formatter::Json));

Config::new().output(multi).init()?;
```

开发时在控制台看到美观的日志，同时写入 JSON 格式到文件供后续分析。

## 实现要点

### 1. WorkerGuard 生命周期管理

**关键点**：返回的 `WorkerGuard` 必须在整个程序生命周期内持有。

文档说明：
```rust
/// Returns a WorkerGuard that MUST be held for the lifetime of your application.
/// Dropping the guard will flush and close the logging backend.
///
/// # Example
/// ```
/// fn main() -> xlog::Result<()> {
///     let _guard = xlog::quick_init()?;
///     // _guard lives until end of main
///     log::info!("Application started");
///     Ok(())
/// }
/// ```
```

### 2. 防止重复初始化

使用 `std::sync::Once` 确保全局订阅者只初始化一次：

```rust
static INIT: Once = Once::new();

pub(crate) fn initialize(config: Config) -> Result<WorkerGuard> {
    let mut result = None;
    let mut error = None;

    INIT.call_once(|| {
        match do_init(config) {
            Ok(guard) => result = Some(guard),
            Err(e) => error = Some(e),
        }
    });

    if INIT.is_completed() && result.is_none() {
        return Err(XlogError::AlreadyInitialized);
    }

    result.ok_or_else(|| error.unwrap())
}
```

### 3. 错误处理策略

所有可能的失败点都转换为 `XlogError`：
- 文件创建失败 → `XlogError::Io`
- 无效配置 → `XlogError::InvalidConfig`
- 重复初始化 → `XlogError::AlreadyInitialized`
- tracing 初始化失败 → `XlogError::InitFailed`

### 4. 性能优化

- **非阻塞写入**：使用 `tracing_appender::non_blocking` 避免阻塞主线程
- **批量刷新**：日志先写入缓冲区，异步批量刷新到磁盘
- **零分配格式化**（可能）：JSON 格式化使用 `serde_json` 的高效序列化
- **懒初始化**：只在首次日志输出时进行昂贵的格式化操作

### 5. 测试策略

**单元测试**：
- `error.rs`: 错误类型转换和消息
- `level.rs`: 字符串解析和 LevelFilter 转换
- `output.rs`: Builder 模式和输出配置
- `formatter.rs`: 各种格式化器的输出

**集成测试**：
- 完整的初始化流程
- 多输出写入验证
- 文件轮转验证
- 重复初始化错误

**示例代码** (`examples/`)：
- `simple.rs`: 最简单的使用
- `file_logging.rs`: 文件日志和轮转
- `multi_output.rs`: 多输出示例
- `json_format.rs`: JSON 格式示例
- `advanced_filtering.rs`: 模块过滤和速率限制

## 依赖更新

```toml
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
json = ["serde", "serde_json"]
```

**新依赖说明**：
- `thiserror`: 优雅的错误类型定义
- `serde` + `serde_json`: JSON 格式化支持（可选 feature）

## 迁移指南

### 从 0.1.x 迁移到 0.2.0

**旧 API**（仍可用，已标记 deprecated）：
```rust
let _guard = xlog::init("./logs", "info")?;
let _guard = xlog::builder()
    .log_home("./logs")
    .log_level("info")
    .build()?;
```

**新 API**（推荐）：
```rust
let _guard = xlog::init_file("./logs", Level::Info)?;
let _guard = xlog::Config::new()
    .output(Output::file("./logs"))
    .level(Level::Info)
    .init()?;
```

**破坏性变更**：
1. `Builder<'a>` 被 `Config` 替代
2. 字符串日志级别被 `Level` 枚举替代（但仍支持 `Level::from_str()`）
3. 错误类型从 `Box<dyn Error>` 改为 `XlogError`

## 实施计划

### 阶段 1: 基础重构
1. 创建新模块结构
2. 实现错误类型和日志级别
3. 实现基础 Config 和 Output

### 阶段 2: 核心功能
4. 实现各种格式化器
5. 实现初始化逻辑
6. 添加快捷函数

### 阶段 3: 高级功能
7. 添加模块过滤
8. 添加速率限制
9. 实现多输出支持

### 阶段 4: 完善和发布
10. 编写测试和示例
11. 更新文档和 README
12. 发布 0.2.0 版本

## 预期成果

1. **更好的用户体验**：简单场景零配置，复杂场景完全可控
2. **更安全的代码**：类型安全的 API，明确的错误处理
3. **更强的功能**：满足从 CLI 到微服务的各种场景
4. **更高的质量**：移除 `#![allow(dead_code)]`，完整的测试覆盖

## 参考资料

- [tracing 文档](https://docs.rs/tracing/)
- [tracing-subscriber 文档](https://docs.rs/tracing-subscriber/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
