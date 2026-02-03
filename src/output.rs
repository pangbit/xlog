use crate::formatter::Formatter;
use std::path::PathBuf;

/// 日志文件轮转策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    /// 不轮转
    Never,
    /// 每小时轮转一次
    Hourly,
    /// 每天轮转一次（默认）
    Daily,
    /// 每周轮转一次
    ///
    /// 注意：tracing-appender 不支持 weekly 轮转，
    /// 实际会使用 daily 轮转作为替代。
    /// 如需精确的周轮转，请使用外部日志轮转工具。
    Weekly,
}

impl From<Rotation> for tracing_appender::rolling::Rotation {
    fn from(rotation: Rotation) -> Self {
        match rotation {
            Rotation::Never => Self::NEVER,
            Rotation::Hourly => Self::HOURLY,
            Rotation::Daily => Self::DAILY,
            Rotation::Weekly => Self::DAILY, // tracing-appender 不支持 weekly，用 daily 代替
        }
    }
}

/// 日志输出目标
#[derive(Debug, Default, Clone)]
pub enum Output {
    /// 标准输出
    #[default]
    Stdout,
    /// 标准错误输出
    Stderr,
    /// 文件输出
    File {
        /// 日志目录路径
        path: PathBuf,
        /// 轮转策略
        rotation: Rotation,
        /// 最大保留文件数
        max_files: usize,
    },
    /// 多个输出目标
    Multi(Vec<OutputWithFormatter>),
}

impl Output {
    /// 创建标准输出目标
    pub fn stdout() -> Self {
        Self::Stdout
    }

    /// 创建标准错误输出目标
    pub fn stderr() -> Self {
        Self::Stderr
    }

    /// 创建文件输出 Builder
    pub fn file<P: Into<PathBuf>>(path: P) -> FileOutputBuilder {
        FileOutputBuilder {
            path: path.into(),
            rotation: Rotation::Daily,
            max_files: 7,
        }
    }

    /// 创建多输出 Builder
    pub fn multi() -> MultiOutputBuilder {
        MultiOutputBuilder::new()
    }

    /// 关联格式化器
    pub fn with_formatter(self, formatter: Formatter) -> OutputWithFormatter {
        match self {
            Output::Stdout => OutputWithFormatter::Stdout { formatter },
            Output::Stderr => OutputWithFormatter::Stderr { formatter },
            Output::File {
                path,
                rotation,
                max_files,
            } => OutputWithFormatter::File {
                path,
                rotation,
                max_files,
                formatter,
            },
            Output::Multi(_) => {
                panic!("Multi output 不支持 with_formatter，请在每个子输出上调用")
            }
        }
    }
}

/// 带格式化器的输出
#[derive(Debug, Clone)]
pub enum OutputWithFormatter {
    /// 标准输出
    Stdout { formatter: Formatter },
    /// 标准错误输出
    Stderr { formatter: Formatter },
    /// 文件输出
    File {
        /// 日志目录路径
        path: PathBuf,
        /// 轮转策略
        rotation: Rotation,
        /// 最大保留文件数
        max_files: usize,
        /// 格式化器
        formatter: Formatter,
    },
}

/// 文件输出 Builder
#[derive(Debug)]
pub struct FileOutputBuilder {
    path: PathBuf,
    rotation: Rotation,
    max_files: usize,
}

impl FileOutputBuilder {
    /// 设置轮转策略
    pub fn with_rotation(mut self, rotation: Rotation) -> Self {
        self.rotation = rotation;
        self
    }

    /// 设置最大保留文件数
    pub fn max_files(mut self, max: usize) -> Self {
        self.max_files = max;
        self
    }

    /// 构建输出目标
    pub fn build(self) -> Output {
        Output::File {
            path: self.path,
            rotation: self.rotation,
            max_files: self.max_files,
        }
    }
}

/// 多输出 Builder
#[derive(Debug)]
pub struct MultiOutputBuilder {
    outputs: Vec<OutputWithFormatter>,
}

impl Default for MultiOutputBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiOutputBuilder {
    /// 创建新的多输出 Builder
    pub fn new() -> Self {
        Self {
            outputs: Vec::new(),
        }
    }

    /// 添加输出
    pub fn add_output(mut self, output: OutputWithFormatter) -> Self {
        self.outputs.push(output);
        self
    }

    /// 构建多输出
    pub fn build(self) -> Output {
        Output::Multi(self.outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_stdout_output() {
        let output = Output::stdout();
        assert!(matches!(output, Output::Stdout));
    }

    #[test]
    fn test_file_output_builder() {
        let output = Output::file("./logs")
            .with_rotation(Rotation::Hourly)
            .max_files(24)
            .build();

        match output {
            Output::File {
                path,
                rotation,
                max_files,
            } => {
                assert_eq!(path, PathBuf::from("./logs"));
                assert!(matches!(rotation, Rotation::Hourly));
                assert_eq!(max_files, 24);
            }
            _ => panic!("Expected File output"),
        }
    }

    #[test]
    fn test_multi_output() {
        let multi = Output::multi()
            .add_output(Output::stdout().with_formatter(Formatter::Pretty))
            .add_output(Output::stderr().with_formatter(Formatter::Json))
            .build();

        match multi {
            Output::Multi(outputs) => {
                assert_eq!(outputs.len(), 2);
            }
            _ => panic!("Expected Multi output"),
        }
    }

    #[test]
    fn test_multi_output_with_formatters() {
        let multi = Output::multi()
            .add_output(Output::stdout().with_formatter(Formatter::Pretty))
            .add_output(Output::stderr().with_formatter(Formatter::Json))
            .add_output(
                Output::file("./logs")
                    .with_rotation(Rotation::Daily)
                    .max_files(7)
                    .build()
                    .with_formatter(Formatter::Compact),
            )
            .build();

        match multi {
            Output::Multi(outputs) => {
                assert_eq!(outputs.len(), 3);

                // Check stdout
                assert!(matches!(
                    outputs[0],
                    OutputWithFormatter::Stdout {
                        formatter: Formatter::Pretty
                    }
                ));

                // Check stderr
                assert!(matches!(
                    outputs[1],
                    OutputWithFormatter::Stderr {
                        formatter: Formatter::Json
                    }
                ));

                // Check file
                match &outputs[2] {
                    OutputWithFormatter::File {
                        path,
                        rotation,
                        max_files,
                        formatter,
                    } => {
                        assert_eq!(path, &PathBuf::from("./logs"));
                        assert!(matches!(rotation, Rotation::Daily));
                        assert_eq!(*max_files, 7);
                        assert!(matches!(formatter, Formatter::Compact));
                    }
                    _ => panic!("Expected File output"),
                }
            }
            _ => panic!("Expected Multi output"),
        }
    }
}

#[cfg(test)]
mod formatter_tests {
    use super::*;

    #[test]
    fn test_stdout_with_formatter() {
        let output = Output::stdout().with_formatter(Formatter::Json);

        match output {
            OutputWithFormatter::Stdout { formatter } => {
                assert!(matches!(formatter, Formatter::Json));
            }
            _ => panic!("Expected Stdout with formatter"),
        }
    }

    #[test]
    fn test_stderr_with_formatter() {
        let output = Output::stderr().with_formatter(Formatter::Compact);

        match output {
            OutputWithFormatter::Stderr { formatter } => {
                assert!(matches!(formatter, Formatter::Compact));
            }
            _ => panic!("Expected Stderr with formatter"),
        }
    }

    #[test]
    fn test_file_with_formatter() {
        let output = Output::file("./logs")
            .with_rotation(Rotation::Hourly)
            .max_files(24)
            .build()
            .with_formatter(Formatter::Pretty);

        match output {
            OutputWithFormatter::File {
                path,
                rotation,
                max_files,
                formatter,
            } => {
                assert_eq!(path, PathBuf::from("./logs"));
                assert!(matches!(rotation, Rotation::Hourly));
                assert_eq!(max_files, 24);
                assert!(matches!(formatter, Formatter::Pretty));
            }
            _ => panic!("Expected File with formatter"),
        }
    }

    #[test]
    #[should_panic(expected = "Multi output 不支持 with_formatter，请在每个子输出上调用")]
    fn test_multi_with_formatter_panics() {
        let multi = Output::multi()
            .add_output(Output::stdout().with_formatter(Formatter::Pretty))
            .add_output(Output::stderr().with_formatter(Formatter::Json))
            .build();

        let _ = multi.with_formatter(Formatter::Json);
    }

    #[test]
    fn test_formatter_with_default() {
        let output = Output::stdout().with_formatter(Formatter::default());

        match output {
            OutputWithFormatter::Stdout { formatter } => {
                assert!(matches!(formatter, Formatter::Pretty));
            }
            _ => panic!("Expected Stdout with Pretty formatter"),
        }
    }
}
