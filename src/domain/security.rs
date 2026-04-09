//! Типы безопасности: управление секретами, audit logging, ошибки.

use std::fmt;

use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;
use zeroize::Zeroize;

/// Ошибки подсистемы безопасности.
#[derive(Error, Debug)]
pub enum SecurityError {
    /// Секрет не найден в хранилище.
    #[error("Secret not found: {0}")]
    SecretNotFound(String),

    /// Ошибка доступа к хранилищу секретов.
    #[error("Secret store error: {0}")]
    SecretStore(String),

    /// Невалидные входные данные.
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Rate limit превышен.
    #[error("Rate limit exceeded for client: {client_id}")]
    RateLimitExceeded {
        /// Идентификатор клиента.
        client_id: String,
    },

    /// Ошибка подписи запроса.
    #[error("Request signing error: {0}")]
    SigningError(String),
}

/// Безопасная обёртка для чувствительных данных.
///
/// Автоматически очищает память при drop через `zeroize`.
/// Не реализует `Display` и `Debug` для предотвращения случайной утечки.
pub struct Secret<T: Zeroize>(T);

impl<T: Zeroize> Secret<T> {
    /// Создаёт новый секрет.
    #[must_use]
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Возвращает ссылку на значение.
    pub fn expose(&self) -> &T {
        &self.0
    }

    /// Потребляет секрет и возвращает значение.
    ///
    /// # Safety
    ///
    /// Использует `ManuallyDrop` для предотвращения двойного drop.
    /// Это безопасно, т.к. `self` потребляется и не будет дропнут.
    pub fn into_inner(self) -> T {
        let this = std::mem::ManuallyDrop::new(self);
        // SAFETY: We own `this` and are consuming it, so the value is valid.
        // ManuallyDrop prevents the destructor from running, so we can safely
        // read the inner value without double-free.
        unsafe { std::ptr::read(&this.0) }
    }
}

impl<T: Zeroize + Clone> Clone for Secret<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Zeroize + Default> Default for Secret<T> {
    fn default() -> Self {
        Self(T::default())
    }
}

impl<T: Zeroize> Drop for Secret<T> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl<T: Zeroize> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Secret([REDACTED])")
    }
}

/// Провайдер секретов.
///
/// Все реализации должны очищать секреты после использования.
#[async_trait]
pub trait SecretProvider: Send + Sync {
    /// Получает секрет по ключу.
    ///
    /// # Errors
    ///
    /// Возвращает [`SecurityError::SecretNotFound`] если ключ не найден.
    async fn get(&self, key: &str) -> Result<Secret<String>, SecurityError>;

    /// Сохраняет секрет.
    ///
    /// # Errors
    ///
    /// Возвращает [`SecurityError::SecretStore`] при сбое записи.
    async fn set(&self, key: &str, value: Secret<String>) -> Result<(), SecurityError>;
}

/// Действие санитизации при логировании.
#[derive(Debug, Clone, Copy)]
pub enum SanitizationAction {
    /// Полностью замаскировать значение.
    Mask,
    /// Частично замаскировать (показать первые/последние символы).
    PartialMask,
    /// Хэшировать значение.
    Hash,
}

/// Audit-событие.
#[derive(Debug, Clone)]
pub enum AuditEvent {
    /// Был получен доступ к API-ключу.
    ApiKeyAccessed {
        /// Идентификатор ключа (замаскированный).
        key_id: String,
        /// Идентификатор сессии.
        session_id: Uuid,
    },
    /// Были предприняты попытки чувствительного логирования.
    SensitiveDataLogged {
        /// Название поля.
        field: String,
        /// Действие санитизации.
        action: SanitizationAction,
    },
    /// Попытка аутентификации.
    AuthAttempt {
        /// Успешность.
        success: bool,
        /// IP-адрес (если доступен).
        ip: Option<String>,
    },
}

impl AuditEvent {
    /// Возвращает имя события для логирования.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::ApiKeyAccessed { .. } => "api_key_accessed",
            Self::SensitiveDataLogged { .. } => "sensitive_data_logged",
            Self::AuthAttempt { .. } => "auth_attempt",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_debug_does_not_expose_value() {
        let secret = Secret::new("super_secret".to_string());
        let debug_str = format!("{secret:?}");
        assert_eq!(debug_str, "Secret([REDACTED])");
        assert!(!debug_str.contains("super_secret"));
    }

    #[test]
    fn test_secret_expose_and_consume() {
        let secret = Secret::new("key123".to_string());
        assert_eq!(secret.expose(), "key123");
        let inner = secret.into_inner();
        assert_eq!(inner, "key123");
    }

    #[test]
    fn test_secret_clone() {
        let secret = Secret::new("value".to_string());
        let cloned = secret.clone();
        assert_eq!(secret.expose(), cloned.expose());
    }

    #[test]
    fn test_audit_event_names() {
        let evt = AuditEvent::ApiKeyAccessed {
            key_id: "sk-***".to_string(),
            session_id: Uuid::new_v4(),
        };
        assert_eq!(evt.name(), "api_key_accessed");
    }

    #[test]
    fn test_security_error_display() {
        let err = SecurityError::SecretNotFound("api_key".to_string());
        assert_eq!(format!("{err}"), "Secret not found: api_key");

        let err = SecurityError::RateLimitExceeded {
            client_id: "192.168.1.1".to_string(),
        };
        assert_eq!(
            format!("{err}"),
            "Rate limit exceeded for client: 192.168.1.1"
        );
    }
}
