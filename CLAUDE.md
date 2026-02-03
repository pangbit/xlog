# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`xlog` is a Rust logging library wrapper around `tracing` and `tracing-subscriber` that provides simplified log initialization with file rotation and configurable output.

## Commands

### Build and Test
```bash
# Build the library
cargo build

# Run all tests
cargo test

# Run a specific test
cargo test test_localtime
cargo test test_builder

# Build with release optimizations
cargo build --release

# Check code without building
cargo check
```

## Architecture

### Core Design

The library provides two initialization patterns:

1. **Simple initialization** via `init()` - Uses default filename ("main.log") and max log files (3)
2. **Builder pattern** via `builder()` - Allows full customization of log configuration

### Key Components

**src/log.rs** - Contains all logging logic:
- `init()`: Quick setup with sensible defaults
- `Builder`: Fluent API for custom configuration
- `init_log()`: Internal implementation handling both stdout and file logging

### Critical Implementation Details

**WorkerGuard Management**: The `WorkerGuard` returned by init functions MUST be held for the lifetime of the application. Dropping it will flush and close the logging backend. Store it at the application level:

```rust
let _guard = xlog::init("./logs", "info")?;
// _guard must live for entire application lifetime
```

**Log Destination Logic**: The special path "stdout" (case-insensitive) routes logs to stdout with ANSI colors. Any other path creates a rolling file appender with:
- Daily rotation (`Rotation::DAILY`)
- Configurable max files (default: 3)
- Configurable filename prefix (default: "main.log")

**Timezone Handling**: Uses `LocalTime::rfc_3339()` for timestamps. Requires the `local-time` feature in `tracing-subscriber` dependency.

**Log Format**: Fixed format includes:
- File path and line number
- RFC 3339 timestamp (local time)
- ANSI colors only for stdout (disabled for files)
- Target logging disabled (`with_target(false)`)

### Dependencies

- `tracing`: Core tracing primitives
- `tracing-appender`: File rotation and non-blocking I/O
- `tracing-subscriber`: Subscriber implementation with `env-filter`, `local-time`, and `time` features

### Public API Surface

Only two functions exposed from `src/lib.rs`:
- `xlog::init(log_home, log_level)`
- `xlog::builder()` → `Builder`

All internal implementation hidden in private module.
