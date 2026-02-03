use xlog::{Config, Level, Output, XlogError};

#[test]
fn test_module_filter_integration() {
    // 测试配置可以成功构建和初始化
    // 注意：由于 Once 模式，只有第一个测试会真正初始化
    let result = Config::new()
        .level(Level::Info)
        .filter_module("test_module", Level::Warn)
        .output(Output::Stdout)
        .init();

    // 可能返回 Ok 或 AlreadyInitialized，两者都是合法的
    assert!(result.is_ok() || matches!(result, Err(XlogError::AlreadyInitialized)));
}

#[test]
fn test_custom_directive_integration() {
    // 测试自定义指令配置可以成功构建
    let result = Config::new()
        .add_directive("test::sub=debug")
        .output(Output::Stdout)
        .init();

    // 可能返回 Ok 或 AlreadyInitialized，两者都是合法的
    assert!(result.is_ok() || matches!(result, Err(XlogError::AlreadyInitialized)));
}
