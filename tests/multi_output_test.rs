use xlog::{Config, Formatter, Level, Output, Result};

#[test]
fn test_multi_output_initialization() -> Result<()> {
    // Create a multi-output configuration
    let multi_output = Output::multi()
        .add_output(Output::stdout().with_formatter(Formatter::Compact))
        .add_output(Output::stderr().with_formatter(Formatter::Json))
        .build();

    let _guard = Config::new()
        .level(Level::Info)
        .output(multi_output)
        .init()?;

    // Log some messages
    tracing::info!("Test message to multi output");
    tracing::warn!("Warning message");

    Ok(())
}

// Note: The following tests are commented out because the logger can only be initialized once
// In real usage, these scenarios would be tested separately or with proper test isolation

// #[test]
// fn test_multi_output_with_file() -> Result<()> {
//     let temp_dir = std::env::temp_dir().join("xlog_test_multi");
//     fs::create_dir_all(&temp_dir).ok();
//
//     let multi_output = Output::multi()
//         .add_output(Output::stdout().with_formatter(Formatter::Pretty))
//         .add_output(
//             Output::file(&temp_dir)
//                 .max_files(3)
//                 .build()
//                 .with_formatter(Formatter::Json),
//         )
//         .build();
//
//     let _guard = Config::new()
//         .level(Level::Debug)
//         .output(multi_output)
//         .init()?;
//
//     tracing::debug!("Debug to multi output");
//     tracing::info!("Info to multi output");
//
//     // Cleanup
//     fs::remove_dir_all(&temp_dir).ok();
//
//     Ok(())
// }

// #[test]
// fn test_multi_guard_drop() -> Result<()> {
//     let temp_dir = std::env::temp_dir().join("xlog_test_guard");
//     fs::create_dir_all(&temp_dir).ok();
//
//     {
//         let multi_output = Output::multi()
//             .add_output(Output::stdout().with_formatter(Formatter::Compact))
//             .add_output(
//                 Output::file(&temp_dir)
//                     .build()
//                     .with_formatter(Formatter::Json),
//             )
//             .build();
//
//         let _guard = Config::new()
//             .level(Level::Info)
//             .output(multi_output)
//             .init()?;
//
//         tracing::info!("Message before guard drop");
//     } // Guard should be dropped here, flushing all outputs
//
//     // Cleanup
//     fs::remove_dir_all(&temp_dir).ok();
//
//     Ok(())
// }
