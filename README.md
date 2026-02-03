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
