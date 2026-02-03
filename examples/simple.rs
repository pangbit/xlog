use xlog::Result;
use tracing::{info, warn, error};

fn main() -> Result<()> {
    // 最简单的使用方式
    let _guard = xlog::quick_init()?;

    info!("Application started");
    warn!("This is a warning");
    error!("This is an error");

    Ok(())
}
