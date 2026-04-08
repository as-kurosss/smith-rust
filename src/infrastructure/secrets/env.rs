//! Провайдер секретов через переменные окружения.

use async_trait::async_trait;

use crate::domain::security::{Secret, SecretProvider, SecurityError};

/// Провайдер, читающий секреты из переменных окружения.
#[derive(Debug, Clone, Default)]
pub struct EnvSecretProvider;

impl EnvSecretProvider {
    /// Создаёт новый экземпляр.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SecretProvider for EnvSecretProvider {
    async fn get(&self, key: &str) -> Result<Secret<String>, SecurityError> {
        std::env::var(key)
            .map(Secret::new)
            .map_err(|_| SecurityError::SecretNotFound(key.to_string()))
    }

    async fn set(&self, _key: &str, _value: Secret<String>) -> Result<(), SecurityError> {
        // EnvSecretProvider — read-only. Запись не поддерживается.
        Err(SecurityError::SecretStore(
            "EnvSecretProvider does not support set".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_env_secret_provider_existing() {
        // Установим переменную окружения для теста
        std::env::set_var("SMITH_TEST_KEY", "test-value");
        let provider = EnvSecretProvider::new();

        let secret = provider
            .get("SMITH_TEST_KEY")
            .await
            .expect("get should succeed");
        assert_eq!(secret.expose(), "test-value");

        // Очистим
        std::env::remove_var("SMITH_TEST_KEY");
    }

    #[tokio::test]
    async fn test_env_secret_provider_missing() {
        let provider = EnvSecretProvider::new();
        let result = provider.get("NONEXISTENT_ENV_VAR_12345").await;
        assert!(matches!(result, Err(SecurityError::SecretNotFound(_))));
    }

    #[tokio::test]
    async fn test_env_secret_provider_set_not_supported() {
        let provider = EnvSecretProvider::new();
        let result = provider.set("test", Secret::new("value".to_string())).await;
        assert!(matches!(result, Err(SecurityError::SecretStore(_))));
    }
}
