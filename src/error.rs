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

    /// Ошибка аутентификации (401/403).
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Rate limit превышен (429).
    #[error("Rate limited by upstream service. Retry after: {retry_after:?}")]
    RateLimited {
        /// Заголовок Retry-After от сервера (в секундах), если доступен.
        retry_after: Option<u64>,
    },

    /// Ошибка upstream-сервера (5xx).
    #[error("Upstream server error: {message} (status: {status_code})")]
    UpstreamError {
        /// Сообщение об ошибке от сервера.
        message: String,
        /// HTTP-статус ответа.
        status_code: u16,
    },

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

    #[test]
    fn test_error_display_authentication_failed() {
        let err = SmithError::AuthenticationFailed("invalid key".to_string());
        assert_eq!(format!("{err}"), "Authentication failed: invalid key");
    }

    #[test]
    fn test_error_display_rate_limited_with_retry() {
        let err = SmithError::RateLimited {
            retry_after: Some(30),
        };
        assert_eq!(
            format!("{err}"),
            "Rate limited by upstream service. Retry after: Some(30)"
        );
    }

    #[test]
    fn test_error_display_rate_limited_no_retry() {
        let err = SmithError::RateLimited { retry_after: None };
        assert_eq!(
            format!("{err}"),
            "Rate limited by upstream service. Retry after: None"
        );
    }

    #[test]
    fn test_error_display_upstream_error() {
        let err = SmithError::UpstreamError {
            message: "internal error".to_string(),
            status_code: 500,
        };
        assert_eq!(
            format!("{err}"),
            "Upstream server error: internal error (status: 500)"
        );
    }
}
