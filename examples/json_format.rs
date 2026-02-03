use tracing::info;
use xlog::{Config, Formatter, Level, Output, Result};

fn main() -> Result<()> {
    let _guard = Config::new()
        .level(Level::Info)
        .output(Output::stdout())
        .formatter(Formatter::json())
        .init()?;

    info!("This is structured JSON log");
    info!(user_id = 123, action = "login", "User logged in");

    Ok(())
}
