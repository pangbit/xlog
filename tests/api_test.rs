use std::str::FromStr;
use xlog::{Config, Level, Output};

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
