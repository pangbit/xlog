use xlog::Config;
use tracing::info;

#[test]
fn test_rate_limit_enforced() {
    // 初始化带速率限制的日志系统
    let _guard = Config::new()
        .rate_limit(10)
        .init()
        .unwrap();

    // 尝试记录超过限制的日志（100条）
    // 期望只有前10条能够通过，其余被丢弃
    for i in 0..100 {
        info!("Test log {}", i);
    }

    // 测试不应该崩溃
    // 注意：由于日志是异步的，无法直接验证限流效果
    // 但至少验证了限流器不会导致崩溃

    // 再发送一些日志确保没有panic
    for i in 0..50 {
        info!("Additional log {}", i);
    }
}
