use xlog::Config;
use tracing::info;

#[test]
fn test_rate_limit_basic() {
    let _guard = Config::new()
        .rate_limit(100)
        .init()
        .unwrap();

    // 生成日志验证不崩溃
    for i in 0..200 {
        info!("Log {}", i);
    }
}
