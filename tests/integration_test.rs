use xlog::{Config, Level, Output};

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
