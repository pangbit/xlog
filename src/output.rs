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
#[derive(Debug, Default)]
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
    Multi(Vec<Output>),
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
        MultiOutputBuilder {
            outputs: Vec::new(),
        }
    }
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
    outputs: Vec<Output>,
}

impl MultiOutputBuilder {
    /// 添加一个输出目标
    pub fn push(mut self, output: Output) -> Self {
        self.outputs.push(output);
        self
    }

    /// 构建多输出目标
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
            .push(Output::stdout())
            .push(Output::stderr())
            .build();

        match multi {
            Output::Multi(outputs) => {
                assert_eq!(outputs.len(), 2);
            }
            _ => panic!("Expected Multi output"),
        }
    }
}
