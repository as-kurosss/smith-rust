//! Retry policy с экспоненциальной задержкой и jitter.
//!
//! Алгоритм: `delay = min(initial * base^attempt + jitter, max_delay)`
//! Jitter: ±25% равномерного распределения для thundering herd mitigation.

use std::future::Future;

use tokio::time::sleep;
use tracing::{debug, warn};

use crate::domain::observability::RetryPolicy;
use crate::error::Result;

/// Выполняет операцию с retry policy.
///
/// # Arguments
///
/// * `policy` — конфигурация retry.
/// * `operation` — асинхронная операция.
///
/// # Errors
///
/// Возвращает последнюю ошибку после исчерпания попыток.
pub async fn with_retry<T, F, Fut>(policy: &RetryPolicy, operation: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut last_error = None;

    for attempt in 0..policy.max_attempts {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(e) => {
                if !policy.is_retryable(&e) {
                    debug!(error = %e, attempt, "non-retryable error, failing fast");
                    return Err(e);
                }
                warn!(
                    error = %e,
                    attempt = attempt + 1,
                    max_attempts = policy.max_attempts,
                    "retryable error, will retry"
                );
                last_error = Some(e);

                if attempt + 1 < policy.max_attempts {
                    let delay = policy.calculate_delay(attempt);
                    debug!(delay_ms = delay.as_millis(), "backing off before retry");
                    sleep(delay).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        crate::error::SmithError::LLM("unexpected retry exhaustion".to_string())
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_with_retry_succeeds_on_first_attempt() {
        let policy = RetryPolicy::new(3, 10, 1000);
        let counter = AtomicU32::new(0);
        let result = with_retry(&policy, || async {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(42)
        })
        .await;
        assert_eq!(result.expect("should succeed"), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_with_retry_succeeds_after_failures() {
        let policy = RetryPolicy::new(3, 1, 100);
        let counter = AtomicU32::new(0);
        let result = with_retry(&policy, || async {
            let count = counter.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                Err(crate::error::SmithError::LLM("temporary".to_string()))
            } else {
                Ok("success")
            }
        })
        .await;
        assert_eq!(result.expect("should succeed after retries"), "success");
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_with_retry_exhausted() {
        let policy = RetryPolicy::new(2, 1, 100);
        let result = with_retry(&policy, || async {
            Err::<(), _>(crate::error::SmithError::LLM("always fails".to_string()))
        })
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_with_retry_non_retryable_fails_fast() {
        let policy = RetryPolicy::new(5, 1, 100);
        let counter = AtomicU32::new(0);
        let result = with_retry(&policy, || async {
            counter.fetch_add(1, Ordering::SeqCst);
            Err::<(), _>(crate::error::SmithError::InvalidInput("bad".to_string()))
        })
        .await;
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1); // only one attempt
    }
}
