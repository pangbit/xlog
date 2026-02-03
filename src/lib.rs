mod error;
mod level;
mod log;
mod output;
mod formatter;
mod config;
mod init;

// 导出公共类型
pub use error::{XlogError, Result};
pub use level::Level;
pub use output::{Output, Rotation, FileOutputBuilder, MultiOutputBuilder};
pub use formatter::Formatter;
pub use config::Config;

pub use log::builder;
pub use log::init;
