use tracing::info;
use xlog::{Config, Level, Result};

fn main() -> Result<()> {
    // 限制每秒最多 1000 条日志
    let _guard = Config::new().level(Level::Info).rate_limit(1000).init()?;

    // 尝试记录大量日志
    for i in 0..5000 {
        info!("Log message {}", i);
    }

    println!("Logged 5000 messages with 1000/s rate limit");

    Ok(())
}
