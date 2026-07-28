//! Token 承载类型：Debug 打码，避免格式化输出时泄露。

/// 一个 Bearer token。`Debug` 永远打印 `Token(***)`，不暴露内容。
#[derive(Clone)]
pub struct Token(String);

impl Token {
    pub fn new(s: String) -> Self {
        Token(s)
    }

    /// 只在注入 Authorization header 的那一刻取用。
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Token(***)")
    }
}
