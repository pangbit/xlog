/// 日志格式化器
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Formatter {
    /// 紧凑格式 - 单行输出，无颜色，适合生产环境
    Compact,
    /// 美观格式 - 多行格式，ANSI 颜色，适合开发环境（默认）
    #[default]
    Pretty,
    /// JSON 格式 - 结构化输出，适合日志收集系统
    Json,
}

impl Formatter {
    /// 创建紧凑格式化器
    pub fn compact() -> Self {
        Self::Compact
    }

    /// 创建美观格式化器
    pub fn pretty() -> Self {
        Self::Pretty
    }

    /// 创建 JSON 格式化器
    pub fn json() -> Self {
        Self::Json
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formatter_creation() {
        let compact = Formatter::compact();
        assert!(matches!(compact, Formatter::Compact));

        let pretty = Formatter::pretty();
        assert!(matches!(pretty, Formatter::Pretty));

        let json = Formatter::json();
        assert!(matches!(json, Formatter::Json));
    }

    #[test]
    fn test_formatter_clone() {
        let formatter = Formatter::Pretty;
        let cloned = formatter;
        assert!(matches!(cloned, Formatter::Pretty));
    }
}
