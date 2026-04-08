//! Ошибки, возникающие в приложении smith-rust.

use thiserror::Error;

/// Центральная enumeration всех возможных ошибок.
#[derive(Error, Debug)]
pub enum SmithError {
    /// Ошибка HTTP-клиента (reqwest).
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// Ошибка сериализации / десериализации JSON.
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// Ошибка сериализации YAML.
    #[error("YAML serialization error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// LLM-провайдер вернул ошибку.
    #[error("LLM provider error: {0}")]
    LLM(String),

    /// Ввод пользователя пуст или некорректен.
    #[error("Invalid user input: {0}")]
    InvalidInput(String),

    /// Ошибка чтения из stdin.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Удобный type alias для Result с нашим типом ошибки.
pub type Result<T> = std::result::Result<T, SmithError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_http() {
        // reqwest::Error не создаётся вручную, поэтому проверим другие варианты
        let err = SmithError::LLM("test error".to_string());
        assert_eq!(format!("{err}"), "LLM provider error: test error");
    }

    #[test]
    fn test_error_display_invalid_input() {
        let err = SmithError::InvalidInput("empty string".to_string());
        assert_eq!(format!("{err}"), "Invalid user input: empty string");
    }
}
