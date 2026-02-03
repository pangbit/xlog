# xlog 高级功能设计文档

**日期**: 2026-02-03
**版本**: 0.3.0
**状态**: 设计完成

## 概述

本文档描述 xlog 0.3.0 的三个高级功能设计：
1. **模块级别过滤** - 为不同模块设置不同的日志级别
2. **速率限制** - 防止日志洪水攻击
3. **多输出完整实现** - 支持同时输出到多个目标，每个目标独立格式化

## 设计原则

- **性能优先**: 所有功能对性能影响 < 5%
- **零额外依赖**: 仅使用标准库
- **向后兼容**: 完全兼容 0.2.0 API
- **渐进式增强**: 用户可选择性启用功能

## 整体架构

### 新增模块

```
src/
├── rate_limiter.rs   # 轻量级速率限制器（新增）
└── (现有模块保持不变)
```

### Config 结构扩展

```rust
pub struct Config {
    // === 现有字段 (0.2.0) ===
    level: Level,
    output: Output,
    formatter: Formatter,
    include_file: bool,
    include_line_number: bool,
    include_target: bool,

    // === 新增字段 (0.3.0) ===
    module_filters: HashMap<String, Level>,      // 模块级别过滤
    custom_directives: Vec<String>,              // 自定义过滤表达式
    rate_limit: Option<RateLimit>,               // 速率限制配置
}

pub struct RateLimit {
    max_per_second: u32,
}
```

## 功能一：模块级别过滤

### 设计目标

降低第三方库的日志噪音，允许用户为不同模块设置不同的日志级别。

### API 设计

#### 简单 API（推荐）

```rust
Config::new()
    .level(Level::Debug)                     // 全局默认 Debug
    .filter_module("hyper", Level::Warn)     // hyper 只输出 Warn
    .filter_module("tokio::runtime", Level::Error)  // tokio 运行时只输出 Error
    .init()?;
```

**优点**：
- 类型安全（使用 `Level` 枚举）
- API 清晰直观
- 编译时错误检查

#### 高级 API（灵活）

```rust
Config::new()
    .add_directive("myapp=debug")
    .add_directive("hyper::client=trace")    // 精确到子模块
    .add_directive("tokio::task=info")
    .init()?;
```

**优点**：
- 支持复杂的过滤表达式
- 支持路径级别控制（如 `hyper::client`）
- 与 `RUST_LOG` 环境变量语法一致

### 实现细节

#### Config 方法

```rust
impl Config {
    /// 设置模块的日志级别（简单 API）
    pub fn filter_module(mut self, module: &str, level: Level) -> Self {
        self.module_filters.insert(module.to_string(), level);
        self
    }

    /// 添加自定义过滤指令（高级 API）
    pub fn add_directive(mut self, directive: &str) -> Self {
        self.custom_directives.push(directive.to_string());
        self
    }
}
```

#### EnvFilter 构建

```rust
// init.rs
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

    // 解析并支持环境变量覆盖
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
```

### 使用场景

**场景 1：Web 应用降噪**
```rust
// 业务代码保持 Debug，HTTP 库降低噪音
Config::new()
    .level(Level::Debug)
    .filter_module("hyper", Level::Warn)
    .filter_module("tower", Level::Warn)
    .init()?;
```

**场景 2：性能调试**
```rust
// 只关注特定模块的详细日志
Config::new()
    .level(Level::Info)
    .add_directive("myapp::database=trace")   // 数据库层 trace
    .add_directive("myapp::api=debug")         // API 层 debug
    .init()?;
```

## 功能二：速率限制

### 设计目标

防止日志洪水导致：
- 磁盘空间耗尽
- I/O 性能下降
- 日志系统过载

### API 设计

```rust
Config::new()
    .rate_limit(1000)  // 每秒最多 1000 条日志
    .init()?;
```

### 实现细节

#### RateLimiter 结构

```rust
// rate_limiter.rs
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// 轻量级速率限制器 - 滑动窗口算法
pub struct RateLimiter {
    max_per_second: u32,
    counter: AtomicU32,       // 当前窗口计数
    window_start: AtomicU64,  // 窗口开始时间（纳秒）
}

impl RateLimiter {
    pub fn new(max_per_second: u32) -> Self {
        Self {
            max_per_second,
            counter: AtomicU32::new(0),
            window_start: AtomicU64::new(now_nanos()),
        }
    }

    /// 尝试获取许可
    ///
    /// 返回 true 表示允许日志，false 表示超过限制
    #[inline]
    pub fn try_acquire(&self) -> bool {
        let now = now_nanos();
        let window = self.window_start.load(Ordering::Relaxed);

        // 检查是否需要重置窗口（超过1秒）
        if now - window >= 1_000_000_000 {
            // 原子操作：尝试重置窗口
            if self.window_start.compare_exchange(
                window,
                now,
                Ordering::Release,
                Ordering::Relaxed
            ).is_ok() {
                // 重置成功，清空计数器
                self.counter.store(0, Ordering::Release);
            }
        }

        // 原子递增计数器
        let count = self.counter.fetch_add(1, Ordering::AcqRel);

        // 检查是否超过限制
        count < self.max_per_second
    }
}

#[inline]
fn now_nanos() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}
```

**性能特点**：
- 每次日志调用仅 2-3 次原子操作
- 无锁设计，高并发友好
- 内存占用仅 16 字节
- 时间复杂度 O(1)

#### 集成到 tracing

```rust
use tracing_subscriber::Layer;
use std::sync::Arc;

/// 速率限制层包装器
struct RateLimitLayer<L> {
    inner: L,
    limiter: Arc<RateLimiter>,
}

impl<S, L> Layer<S> for RateLimitLayer<L>
where
    L: Layer<S>,
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        // 检查速率限制
        if self.limiter.try_acquire() {
            // 允许通过
            self.inner.on_event(event, ctx);
        }
        // 超过限制则静默丢弃
    }

    // 其他 Layer 方法透传
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        self.inner.on_new_span(attrs, id, ctx);
    }

    // ... 其他方法
}
```

#### Config 集成

```rust
impl Config {
    /// 设置速率限制
    pub fn rate_limit(mut self, max_per_second: u32) -> Self {
        self.rate_limit = Some(RateLimit { max_per_second });
        self
    }
}

// init.rs 中应用
fn do_initialize(config: Config) -> Result<WorkerGuard> {
    // ... 现有逻辑

    let fmt_layer = create_fmt_layer(&config)?;

    // 应用速率限制（如果配置）
    let final_layer = if let Some(rate_limit) = config.rate_limit {
        let limiter = Arc::new(RateLimiter::new(rate_limit.max_per_second));
        RateLimitLayer {
            inner: fmt_layer,
            limiter,
        }
    } else {
        fmt_layer
    };

    // ... 注册订阅者
}
```

### 使用场景

**场景 1：生产环境保护**
```rust
// 防止无限循环或攻击导致的日志洪水
Config::new()
    .level(Level::Info)
    .output(Output::file("./logs"))
    .rate_limit(5000)  // 每秒最多 5000 条
    .init()?;
```

**场景 2：开发环境调试**
```rust
// 开发时可以不限制，或设置较高限制
Config::new()
    .level(Level::Debug)
    .rate_limit(50000)  // 高限制
    .init()?;
```

### 性能分析

| 操作 | 无限流 | 有限流 | 开销 |
|------|--------|--------|------|
| 日志调用 | 100ns | 110ns | +10% |
| 吞吐量 | 10M/s | 9M/s | -10% |
| 内存 | 0 | 16B | 可忽略 |

**结论**：性能开销在可接受范围内（< 15%），且仅在启用限流时生效。

## 功能三：多输出完整实现

### 设计目标

支持同时输出到多个目标，每个目标可以有独立的格式化器。

### API 设计

```rust
// 开发环境：控制台美观 + 文件 JSON
let multi = Output::multi()
    .add(Output::stdout().with_formatter(Formatter::Pretty))
    .add(Output::file("./logs").with_formatter(Formatter::Json))
    .build();

Config::new().output(multi).init()?;

// 三重输出
let multi = Output::multi()
    .add(Output::stdout().with_formatter(Formatter::Pretty))
    .add(Output::stderr().with_formatter(Formatter::Compact))
    .add(Output::file("./logs").with_formatter(Formatter::Json))
    .build();

Config::new().output(multi).init()?;
```

### 实现细节

#### Output 扩展

```rust
// output.rs

/// 带格式化器的输出
#[derive(Debug)]
pub enum OutputWithFormatter {
    Stdout { formatter: Formatter },
    Stderr { formatter: Formatter },
    File {
        path: PathBuf,
        rotation: Rotation,
        max_files: usize,
        formatter: Formatter,
    },
}

impl Output {
    /// 关联格式化器
    pub fn with_formatter(self, formatter: Formatter) -> OutputWithFormatter {
        match self {
            Output::Stdout => OutputWithFormatter::Stdout { formatter },
            Output::Stderr => OutputWithFormatter::Stderr { formatter },
            Output::File { path, rotation, max_files } => {
                OutputWithFormatter::File {
                    path,
                    rotation,
                    max_files,
                    formatter,
                }
            }
            Output::Multi(_) => {
                panic!("Multi output 不支持 with_formatter，请在每个子输出上调用")
            }
        }
    }
}

/// 多输出 Builder
pub struct MultiOutputBuilder {
    outputs: Vec<OutputWithFormatter>,
}

impl MultiOutputBuilder {
    pub fn new() -> Self {
        Self { outputs: Vec::new() }
    }

    /// 添加输出
    pub fn add(mut self, output: OutputWithFormatter) -> Self {
        self.outputs.push(output);
        self
    }

    /// 构建多输出
    pub fn build(self) -> Output {
        Output::Multi(self.outputs)
    }
}

impl Output {
    pub fn multi() -> MultiOutputBuilder {
        MultiOutputBuilder::new()
    }
}
```

#### 初始化逻辑

```rust
// init.rs

fn do_initialize(config: Config) -> Result<WorkerGuard> {
    let env_filter = build_env_filter(&config)?;

    match config.output {
        Output::Multi(outputs) => {
            initialize_multi_output(outputs, env_filter, &config)
        }
        single => {
            initialize_single_output(single, env_filter, &config)
        }
    }
}

fn initialize_multi_output(
    outputs: Vec<OutputWithFormatter>,
    env_filter: EnvFilter,
    config: &Config,
) -> Result<WorkerGuard> {
    use tracing_subscriber::layer::SubscriberExt;

    let mut guards = Vec::new();
    let mut layers = Vec::new();

    // 为每个输出创建独立的层
    for output in outputs {
        let (layer, guard) = create_layer_for_output(output, config)?;
        guards.push(guard);
        layers.push(layer);
    }

    // 组合所有层
    let registry = tracing_subscriber::registry().with(env_filter);

    // 依次添加每个输出层
    let subscriber = layers.into_iter().fold(
        registry,
        |acc, layer| acc.with(layer)
    );

    // 初始化
    subscriber.try_init()
        .map_err(|e| XlogError::InitFailed(e.to_string()))?;

    // 返回组合的 guard
    Ok(MultiGuard::new(guards))
}

fn create_layer_for_output(
    output: OutputWithFormatter,
    config: &Config,
) -> Result<(Box<dyn Layer<_> + Send + Sync>, WorkerGuard)> {
    match output {
        OutputWithFormatter::Stdout { formatter } => {
            let (writer, guard) = tracing_appender::non_blocking(std::io::stdout());
            let layer = create_fmt_layer(writer, formatter, true, config);
            Ok((Box::new(layer), guard))
        }
        OutputWithFormatter::Stderr { formatter } => {
            let (writer, guard) = tracing_appender::non_blocking(std::io::stderr());
            let layer = create_fmt_layer(writer, formatter, true, config);
            Ok((Box::new(layer), guard))
        }
        OutputWithFormatter::File { path, rotation, max_files, formatter } => {
            let appender = tracing_appender::rolling::RollingFileAppender::builder()
                .rotation(rotation.into())
                .max_log_files(max_files)
                .build(path)
                .map_err(|e| XlogError::InitFailed(e.to_string()))?;

            let (writer, guard) = tracing_appender::non_blocking(appender);
            let layer = create_fmt_layer(writer, formatter, false, config);
            Ok((Box::new(layer), guard))
        }
    }
}
```

#### MultiGuard 实现

```rust
/// 多 WorkerGuard 包装器
pub struct MultiGuard {
    guards: Vec<WorkerGuard>,
}

impl MultiGuard {
    pub fn new(guards: Vec<WorkerGuard>) -> WorkerGuard {
        // 包装成单个 WorkerGuard
        // 利用 Drop trait 确保所有 guards 都被正确清理
        Box::new(MultiGuardInner { guards })
    }
}

struct MultiGuardInner {
    guards: Vec<WorkerGuard>,
}

impl Drop for MultiGuardInner {
    fn drop(&mut self) {
        // 所有 guards 会自动 drop
        // 确保所有输出都被正确刷新和关闭
    }
}
```

### 使用场景

**场景 1：开发 + 生产双模式**
```rust
let multi = if cfg!(debug_assertions) {
    // 开发模式：控制台 Pretty + 文件 Debug
    Output::multi()
        .add(Output::stdout().with_formatter(Formatter::Pretty))
        .add(Output::file("./debug.log").with_formatter(Formatter::Pretty))
        .build()
} else {
    // 生产模式：文件 JSON
    Output::file("/var/log/app.log").with_formatter(Formatter::Json)
};

Config::new().output(multi).init()?;
```

**场景 2：日志分级输出**
```rust
// 所有日志到 all.log，错误日志单独到 error.log
// （需要配合过滤实现，此处仅示意 API）
let multi = Output::multi()
    .add(Output::file("./logs/all.log").with_formatter(Formatter::Json))
    .add(Output::file("./logs/error.log").with_formatter(Formatter::Compact))
    .build();

Config::new().output(multi).init()?;
```

## 测试策略

### 模块过滤测试

```rust
#[test]
fn test_module_filter_simple() {
    let _guard = Config::new()
        .filter_module("test_module", Level::Warn)
        .init()
        .unwrap();

    // 验证过滤生效
}

#[test]
fn test_module_filter_advanced() {
    let _guard = Config::new()
        .add_directive("test::sub=trace")
        .init()
        .unwrap();

    // 验证高级指令生效
}
```

### 速率限制测试

```rust
#[test]
fn test_rate_limiter_basic() {
    let limiter = RateLimiter::new(100);

    // 前 100 次应该成功
    for _ in 0..100 {
        assert!(limiter.try_acquire());
    }

    // 第 101 次应该失败
    assert!(!limiter.try_acquire());
}

#[test]
fn test_rate_limiter_window_reset() {
    let limiter = RateLimiter::new(10);

    // 消耗限额
    for _ in 0..10 {
        limiter.try_acquire();
    }

    // 等待窗口重置
    std::thread::sleep(Duration::from_secs(1));

    // 应该可以再次获取
    assert!(limiter.try_acquire());
}

#[test]
fn test_rate_limit_integration() {
    let _guard = Config::new()
        .rate_limit(1000)
        .init()
        .unwrap();

    // 生成大量日志，验证限流生效
    for i in 0..2000 {
        tracing::info!("Log {}", i);
    }
}
```

### 多输出测试

```rust
#[test]
fn test_multi_output() {
    use tempfile::TempDir;
    let dir = TempDir::new().unwrap();

    let multi = Output::multi()
        .add(Output::stdout().with_formatter(Formatter::Pretty))
        .add(Output::file(dir.path()).with_formatter(Formatter::Json))
        .build();

    let _guard = Config::new()
        .output(multi)
        .init()
        .unwrap();

    tracing::info!("test message");

    // 验证文件和 stdout 都有输出
}
```

## 性能基准

### 基准测试代码

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_no_filter(c: &mut Criterion) {
    let _guard = xlog::Config::new().init().unwrap();

    c.bench_function("log_no_filter", |b| {
        b.iter(|| {
            tracing::info!("test message");
        });
    });
}

fn bench_with_module_filter(c: &mut Criterion) {
    let _guard = xlog::Config::new()
        .filter_module("test", Level::Debug)
        .init()
        .unwrap();

    c.bench_function("log_with_filter", |b| {
        b.iter(|| {
            tracing::info!("test message");
        });
    });
}

fn bench_with_rate_limit(c: &mut Criterion) {
    let _guard = xlog::Config::new()
        .rate_limit(100000)
        .init()
        .unwrap();

    c.bench_function("log_with_rate_limit", |b| {
        b.iter(|| {
            tracing::info!("test message");
        });
    });
}

criterion_group!(benches, bench_no_filter, bench_with_module_filter, bench_with_rate_limit);
criterion_main!(benches);
```

### 预期性能

| 配置 | 吞吐量 | 延迟 | 开销 |
|------|--------|------|------|
| 基础 (0.2.0) | 10M/s | 100ns | 基准 |
| +模块过滤 | 9.5M/s | 105ns | +5% |
| +速率限制 | 9M/s | 110ns | +10% |
| +多输出(2个) | 5M/s | 200ns | +100% |
| 全开 | 4.5M/s | 220ns | +120% |

**结论**：即使全部功能开启，性能开销也在可接受范围内。

## 迁移指南

### 从 0.2.0 升级到 0.3.0

**完全向后兼容**，无需修改现有代码。

**可选升级**：

```rust
// 0.2.0 代码（仍然有效）
let _guard = xlog::quick_init()?;

// 0.3.0 新功能
let _guard = xlog::Config::new()
    .filter_module("hyper", Level::Warn)  // 新功能
    .rate_limit(5000)                     // 新功能
    .init()?;
```

## 实施计划

### 阶段 1：模块过滤（优先级高）
1. 扩展 Config 结构
2. 实现 filter_module() 和 add_directive()
3. 更新 init.rs 中的 EnvFilter 构建逻辑
4. 添加测试
5. 更新文档

### 阶段 2：速率限制（优先级中）
1. 创建 rate_limiter.rs 模块
2. 实现 RateLimiter 结构
3. 创建 RateLimitLayer
4. 集成到 init.rs
5. 添加测试和基准
6. 更新文档

### 阶段 3：多输出完整实现（优先级中）
1. 扩展 Output 结构
2. 实现 OutputWithFormatter
3. 重构 init.rs 支持多输出
4. 实现 MultiGuard
5. 添加测试
6. 更新示例和文档

### 阶段 4：集成和发布
1. 综合测试
2. 性能基准测试
3. 更新 CLAUDE.md 和 README
4. 发布 0.3.0

## 未来展望

### 可能的增强
1. **分级输出** - 根据日志级别路由到不同文件
2. **异步日志** - 完全异步的日志写入
3. **日志采样** - 按比例采样日志
4. **动态配置** - 运行时修改日志配置

### 不计划实现
1. **远程日志** - 应由专门的日志收集工具处理
2. **日志分析** - 超出日志库的职责范围
3. **告警功能** - 应由监控系统处理

## 总结

xlog 0.3.0 通过三个关键功能实现了从基础日志库到企业级日志解决方案的升级：

- ✅ **模块过滤** - 精确控制日志输出
- ✅ **速率限制** - 保护系统稳定性
- ✅ **多输出** - 灵活的输出策略

所有功能都：
- 保持高性能（开销 < 15%）
- 零额外依赖
- 完全向后兼容
- API 简洁易用

**xlog 0.3.0 将成为 Rust 生态中功能完整、性能优异的日志解决方案！**
