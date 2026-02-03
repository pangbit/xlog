use xlog::{Config, Level, Result};
use tracing::{debug, info, warn};

fn main() -> Result<()> {
    // 设置全局为 Debug，但降低第三方库噪音
    let _guard = Config::new()
        .level(Level::Debug)
        .filter_module("hyper", Level::Warn)
        .filter_module("tokio", Level::Error)
        .init()?;

    debug!("Application debug message");
    info!("Application info message");
    warn!("Application warning");

    Ok(())
}
