//! In-memory провайдер секретов.
//!
//! Предназначен для тестирования. Не сохраняет секреты между запусками.

use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::domain::security::{Secret, SecretProvider, SecurityError};

/// In-memory хранилище секретов.
#[derive(Debug)]
pub struct MemorySecretProvider {
    store: RwLock<HashMap<String, Secret<String>>>,
}

impl MemorySecretProvider {
    /// Создаёт пустое хранилище.
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemorySecretProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecretProvider for MemorySecretProvider {
    async fn get(&self, key: &str) -> Result<Secret<String>, SecurityError> {
        let store = self.store.read().await;
        store
            .get(key)
            .cloned()
            .ok_or_else(|| SecurityError::SecretNotFound(key.to_string()))
    }

    async fn set(&self, key: &str, value: Secret<String>) -> Result<(), SecurityError> {
        let mut store = self.store.write().await;
        store.insert(key.to_string(), value);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_secret_provider_set_and_get() {
        let provider = MemorySecretProvider::new();
        provider
            .set("api_key", Secret::new("sk-123".to_string()))
            .await
            .expect("set should succeed");

        let secret = provider.get("api_key").await.expect("get should succeed");
        assert_eq!(secret.expose(), "sk-123");
    }

    #[tokio::test]
    async fn test_memory_secret_provider_not_found() {
        let provider = MemorySecretProvider::new();
        let result = provider.get("nonexistent").await;
        assert!(matches!(result, Err(SecurityError::SecretNotFound(_))));
    }

    #[tokio::test]
    async fn test_memory_secret_provider_overwrite() {
        let provider = MemorySecretProvider::new();
        provider
            .set("key", Secret::new("old".to_string()))
            .await
            .expect("set");
        provider
            .set("key", Secret::new("new".to_string()))
            .await
            .expect("set");

        let secret = provider.get("key").await.expect("get");
        assert_eq!(secret.expose(), "new");
    }
}
