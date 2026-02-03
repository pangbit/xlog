use xlog::{Config, Level, Output, Rotation, Result};
use tracing::info;
use std::fs;

fn main() -> Result<()> {
    // 创建日志目录（如果不存在）
    fs::create_dir_all("./logs").map_err(|e| xlog::XlogError::Io(e))?;

    let _guard = Config::new()
        .level(Level::Debug)
        .output(
            Output::file("./logs")
                .with_rotation(Rotation::Daily)
                .max_files(7)
                .build()
        )
        .init()?;

    info!("This message goes to file");
    info!("Logs will rotate daily");

    Ok(())
}
