use tracing::info;
use xlog::{Config, Formatter, Output, Result};

fn main() -> Result<()> {
    // 同时输出到控制台（Pretty）和文件（JSON）
    let multi = Output::multi()
        .add_output(Output::stdout().with_formatter(Formatter::Pretty))
        .add_output(
            Output::file("./logs")
                .build()
                .with_formatter(Formatter::Json),
        )
        .build();

    let _guard = Config::new().output(multi).init()?;

    info!("This message goes to both console and file");
    info!(user = "alice", action = "login", "User logged in");

    Ok(())
}
