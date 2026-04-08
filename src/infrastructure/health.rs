//! Health checker — агрегация статусов компонентов системы.

use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::warn;

use crate::domain::observability::{HealthStatus, SystemHealth};

/// Health checker для мониторинга состояния системы.
pub struct HealthChecker {
    components: Arc<RwLock<SystemHealth>>,
}

impl HealthChecker {
    /// Создаёт новый checker с начальным статусом.
    #[must_use]
    pub fn new() -> Self {
        Self {
            components: Arc::new(RwLock::new(SystemHealth {
                llm: HealthStatus::Healthy,
                storage: HealthStatus::Healthy,
                memory: HealthStatus::Healthy,
            })),
        }
    }

    /// Обновляет статус LLM-провайдера.
    pub async fn update_llm(&self, status: HealthStatus, error: Option<String>) {
        let mut health = self.components.write().await;
        health.llm = status;
        if let Some(ref e) = error {
            warn!(component = "llm", status = %status, error = e, "component health changed");
        }
    }

    /// Обновляет статус хранилища.
    pub async fn update_storage(&self, status: HealthStatus, error: Option<String>) {
        let mut health = self.components.write().await;
        health.storage = status;
        if let Some(ref e) = error {
            warn!(component = "storage", status = %status, error = e, "component health changed");
        }
    }

    /// Обновляет статус памяти.
    pub async fn update_memory(&self, status: HealthStatus, error: Option<String>) {
        let mut health = self.components.write().await;
        health.memory = status;
        if let Some(ref e) = error {
            warn!(component = "memory", status = %status, error = e, "component health changed");
        }
    }

    /// Возвращает текущий агрегированный статус.
    pub async fn check(&self) -> SystemHealth {
        let health = self.components.read().await;
        health.clone()
    }

    /// Проверяет, работает ли система полностью.
    pub async fn is_healthy(&self) -> bool {
        let health = self.check().await;
        health.is_healthy()
    }

    /// Форматирует статус для вывода.
    pub async fn format_status(&self) -> String {
        let health = self.check().await;
        format!(
            "Overall: {}\n  LLM:     {}\n  Storage: {}\n  Memory:  {}",
            health.overall(),
            health.llm,
            health.storage,
            health.memory,
        )
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_checker_initial_state() {
        let checker = HealthChecker::new();
        assert!(checker.is_healthy().await);
    }

    #[tokio::test]
    async fn test_health_checker_update_status() {
        let checker = HealthChecker::new();
        checker
            .update_llm(HealthStatus::Degraded, Some("slow response".to_string()))
            .await;
        let health = checker.check().await;
        assert_eq!(health.llm, HealthStatus::Degraded);
        assert!(!health.is_healthy());
    }

    #[tokio::test]
    async fn test_health_checker_format_status() {
        let checker = HealthChecker::new();
        let status = checker.format_status().await;
        assert!(status.contains("healthy"));
        assert!(status.contains("LLM"));
    }
}
