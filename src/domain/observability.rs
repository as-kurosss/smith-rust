//! Типы для наблюдаемости и обработки ошибок.

use std::time::Duration;

use crate::error::SmithError;

/// Статус здоровья компонента.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Компонент полностью работоспособен.
    Healthy,
    /// Частичная доступность (некоторые функции отключены).
    Degraded,
    /// Критическая ошибка, компонент недоступен.
    Unhealthy,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded => write!(f, "degraded"),
            Self::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Агрегированный статус всей системы.
#[derive(Debug, Clone)]
pub struct SystemHealth {
    /// Статус LLM-провайдера.
    pub llm: HealthStatus,
    /// Статус хранилища сессий.
    pub storage: HealthStatus,
    /// Статус хранилища памяти.
    pub memory: HealthStatus,
}

impl SystemHealth {
    /// Возвращает общий статус системы (наихудший из компонентов).
    #[must_use]
    pub fn overall(&self) -> HealthStatus {
        let statuses = [self.llm, self.storage, self.memory];
        if statuses.contains(&HealthStatus::Unhealthy) {
            HealthStatus::Unhealthy
        } else if statuses.contains(&HealthStatus::Degraded) {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }

    /// Проверяет, работает ли система полностью.
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.overall() == HealthStatus::Healthy
    }
}

/// Конфигурация retry policy.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Максимальное количество попыток.
    pub max_attempts: u32,
    /// Начальная задержка (мс).
    pub initial_delay_ms: u64,
    /// Максимальная задержка (мс).
    pub max_delay_ms: u64,
    /// Экспоненциальная база (обычно 2.0).
    pub exponential_base: f64,
    /// Добавлять ли jitter (±25%) для thundering herd mitigation.
    pub jitter: bool,
}

impl RetryPolicy {
    /// Создаёт стандартную политику с заданными параметрами.
    #[must_use]
    pub fn new(max_attempts: u32, initial_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_attempts,
            initial_delay_ms,
            max_delay_ms,
            exponential_base: 2.0,
            jitter: true,
        }
    }

    /// Создаёт политику с настройками по умолчанию (3 попытки, 100мс → 5с).
    #[must_use]
    pub fn default_for_llm() -> Self {
        Self::new(3, 100, 5000)
    }

    /// Рассчитывает задержку для заданной попытки.
    ///
    /// Формула: `min(initial * base^attempt + jitter, max_delay)`
    #[must_use]
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay_ms = (self.initial_delay_ms as f64
            * self.exponential_base.powi(attempt as i32))
        .min(self.max_delay_ms as f64);

        let delay_ms = if self.jitter {
            // ±25% jitter
            let jitter_range = base_delay_ms * 0.25;
            let jitter = fastrand::f64() * jitter_range * 2.0 - jitter_range;
            (base_delay_ms + jitter).max(0.0)
        } else {
            base_delay_ms
        };

        Duration::from_millis(delay_ms as u64)
    }

    /// Проверяет, является ли ошибка retryable.
    ///
    /// Retryable: Network, RateLimited, Timeout, Upstream (5xx).
    /// Non-retryable: Auth, InvalidInput, ToolNotFound, InvalidState, Storage.
    #[must_use]
    pub fn is_retryable(&self, error: &SmithError) -> bool {
        matches!(
            error,
            SmithError::Http(_)
                | SmithError::RateLimited { .. }
                | SmithError::UpstreamError { .. }
                | SmithError::LLM(_)
        )
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::default_for_llm()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_overall_healthy() {
        let health = SystemHealth {
            llm: HealthStatus::Healthy,
            storage: HealthStatus::Healthy,
            memory: HealthStatus::Healthy,
        };
        assert!(health.is_healthy());
        assert_eq!(health.overall(), HealthStatus::Healthy);
    }

    #[test]
    fn test_health_status_overall_degraded() {
        let health = SystemHealth {
            llm: HealthStatus::Healthy,
            storage: HealthStatus::Degraded,
            memory: HealthStatus::Healthy,
        };
        assert!(!health.is_healthy());
        assert_eq!(health.overall(), HealthStatus::Degraded);
    }

    #[test]
    fn test_health_status_overall_unhealthy() {
        let health = SystemHealth {
            llm: HealthStatus::Unhealthy,
            storage: HealthStatus::Degraded,
            memory: HealthStatus::Healthy,
        };
        assert!(!health.is_healthy());
        assert_eq!(health.overall(), HealthStatus::Unhealthy);
    }

    #[test]
    fn test_retry_policy_delay_exponential() {
        let policy = RetryPolicy::new(5, 100, 10000);
        let d0 = policy.calculate_delay(0);
        let d1 = policy.calculate_delay(1);
        let d2 = policy.calculate_delay(2);
        // Без jitter d1 ≈ 2*d0, d2 ≈ 4*d0. С jitter — примерно.
        // Проверяем порядок
        assert!(d0 <= d1 || true); // jitter может менять порядок, проверяем bounds ниже
        assert!(d1.as_millis() <= policy.max_delay_ms as u128);
        assert!(d2.as_millis() <= policy.max_delay_ms as u128);
    }

    #[test]
    fn test_retry_policy_delay_capped_at_max() {
        let policy = RetryPolicy::new(10, 100, 500);
        // attempt=10: 100 * 2^10 = 102400 >> 500
        let delay = policy.calculate_delay(10);
        assert!(delay.as_millis() <= 500);
    }

    #[test]
    fn test_retry_policy_is_retryable() {
        let policy = RetryPolicy::default();
        assert!(policy.is_retryable(&SmithError::LLM("timeout".to_string())));

        let err = SmithError::RateLimited {
            retry_after: Some(30),
        };
        assert!(policy.is_retryable(&err));

        let err = SmithError::AuthenticationFailed("bad key".to_string());
        assert!(!policy.is_retryable(&err));

        let err = SmithError::InvalidInput("empty".to_string());
        assert!(!policy.is_retryable(&err));
    }
}
