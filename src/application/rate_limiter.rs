//! Per-client rate limiter на основе token bucket.
//!
//! Каждый клиент (по IP или API-ключу) имеет свой bucket
//! с независимым счётчиком токенов.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::Mutex;
use tracing::debug;

use crate::error::SmithError;

/// Конфигурация rate limiter.
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// Максимальное количество токенов (ёмкость).
    pub max_tokens: f64,
    /// Скорость пополнения (токенов в секунду).
    pub refill_rate: f64,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        // 60 запросов в минуту = 1 токен/сек
        Self {
            max_tokens: 60.0,
            refill_rate: 1.0,
        }
    }
}

/// Token bucket для одного клиента.
#[derive(Debug)]
struct ClientBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,
    last_refill: Instant,
}

impl ClientBucket {
    fn new(config: &RateLimiterConfig) -> Self {
        Self {
            tokens: config.max_tokens,
            max_tokens: config.max_tokens,
            refill_rate: config.refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    fn try_consume(&mut self, tokens: f64) -> bool {
        self.refill();
        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    fn remaining(&self) -> f64 {
        self.tokens
    }
}

/// Per-client rate limiter.
pub struct RateLimiter {
    buckets: Arc<Mutex<HashMap<String, ClientBucket>>>,
    config: RateLimiterConfig,
}

impl RateLimiter {
    /// Создаёт новый rate limiter с конфигурацией по умолчанию.
    #[must_use]
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            config: RateLimiterConfig::default(),
        }
    }

    /// Создаёт rate limiter с кастомной конфигурацией.
    #[must_use]
    pub fn with_config(config: RateLimiterConfig) -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Проверяет, может ли клиент выполнить запрос.
    ///
    /// # Errors
    ///
    /// Возвращает [`SmithError::RateLimited`] если лимит превышен.
    pub async fn check_rate_limit(&self, client_id: &str) -> Result<(), SmithError> {
        let mut buckets = self.buckets.lock().await;
        let bucket = buckets
            .entry(client_id.to_string())
            .or_insert_with(|| ClientBucket::new(&self.config));

        if bucket.try_consume(1.0) {
            debug!(
                client_id,
                remaining = bucket.remaining(),
                "rate limit check passed"
            );
            Ok(())
        } else {
            debug!(
                client_id,
                remaining = bucket.remaining(),
                "rate limit exceeded"
            );
            Err(SmithError::RateLimitExceeded {
                client_id: client_id.to_string(),
            })
        }
    }

    /// Возвращает оставшиеся токены для клиента.
    pub async fn remaining_tokens(&self, client_id: &str) -> f64 {
        let buckets = self.buckets.lock().await;
        buckets
            .get(client_id)
            .map(|b| b.remaining())
            .unwrap_or(self.config.max_tokens)
    }

    /// Сбрасывает лимит для клиента.
    pub async fn reset_client(&self, client_id: &str) {
        let mut buckets = self.buckets.lock().await;
        buckets.remove(client_id);
    }

    /// Возвращает количество отслеживаемых клиентов.
    pub async fn client_count(&self) -> usize {
        let buckets = self.buckets.lock().await;
        buckets.len()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_within_limit() {
        let config = RateLimiterConfig {
            max_tokens: 5.0,
            refill_rate: 0.0, // disable refill for test
        };
        let limiter = RateLimiter::with_config(config);

        for _ in 0..5 {
            limiter
                .check_rate_limit("client1")
                .await
                .expect("should pass");
        }

        // 6th request should fail
        let result = limiter.check_rate_limit("client1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_independent_clients() {
        let config = RateLimiterConfig {
            max_tokens: 2.0,
            refill_rate: 0.0,
        };
        let limiter = RateLimiter::with_config(config);

        limiter.check_rate_limit("client1").await.expect("pass");
        limiter.check_rate_limit("client1").await.expect("pass");
        limiter.check_rate_limit("client1").await.expect_err("fail");

        // client2 should be independent
        limiter.check_rate_limit("client2").await.expect("pass");
    }

    #[tokio::test]
    async fn test_rate_limiter_remaining_tokens() {
        let config = RateLimiterConfig {
            max_tokens: 10.0,
            refill_rate: 0.0,
        };
        let limiter = RateLimiter::with_config(config);

        assert_eq!(limiter.remaining_tokens("client1").await, 10.0);
        limiter.check_rate_limit("client1").await.expect("pass");
        assert_eq!(limiter.remaining_tokens("client1").await, 9.0);
    }

    #[tokio::test]
    async fn test_rate_limiter_reset() {
        let config = RateLimiterConfig {
            max_tokens: 2.0,
            refill_rate: 0.0,
        };
        let limiter = RateLimiter::with_config(config);

        limiter.check_rate_limit("client1").await.expect("pass");
        limiter.check_rate_limit("client1").await.expect("pass");
        limiter.check_rate_limit("client1").await.expect_err("fail");

        limiter.reset_client("client1").await;
        limiter
            .check_rate_limit("client1")
            .await
            .expect("pass after reset");
    }

    #[tokio::test]
    async fn test_rate_limiter_client_count() {
        let limiter = RateLimiter::new();
        limiter.check_rate_limit("c1").await.expect("pass");
        limiter.check_rate_limit("c2").await.expect("pass");
        assert_eq!(limiter.client_count().await, 2);
    }
}
