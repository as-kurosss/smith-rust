//! Audit logger — запись security-событий в audit-канал.

use tracing::info;

use crate::domain::security::{AuditEvent, SanitizationAction};

/// Trait для audit-логгеров.
#[async_trait::async_trait]
pub trait AuditLogger: Send + Sync {
    /// Записывает audit-событие.
    async fn log(&self, event: &AuditEvent);
}

/// Реализация audit-логгера через tracing.
///
/// Все события записываются в span `audit` с уровнем INFO.
#[derive(Debug, Clone, Default)]
pub struct TracingAuditLogger;

impl TracingAuditLogger {
    /// Создаёт новый экземпляр.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl AuditLogger for TracingAuditLogger {
    async fn log(&self, event: &AuditEvent) {
        match event {
            AuditEvent::ApiKeyAccessed { key_id, session_id } => {
                info!(
                    target: "smith::audit",
                    event = event.name(),
                    key_id = key_id,
                    session_id = %session_id,
                    "API key accessed"
                );
            }
            AuditEvent::SensitiveDataLogged { field, action } => {
                let action_str = match action {
                    SanitizationAction::Mask => "masked",
                    SanitizationAction::PartialMask => "partially_masked",
                    SanitizationAction::Hash => "hashed",
                };
                info!(
                    target: "smith::audit",
                    event = event.name(),
                    field = field,
                    action = action_str,
                    "Sensitive data logged with sanitization"
                );
            }
            AuditEvent::AuthAttempt { success, ip } => {
                info!(
                    target: "smith::audit",
                    event = event.name(),
                    success,
                    ip = ip.as_deref().unwrap_or("unknown"),
                    "Authentication attempt"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tracing_audit_logger_api_key_accessed() {
        let logger = TracingAuditLogger::new();
        let event = AuditEvent::ApiKeyAccessed {
            key_id: "sk-***".to_string(),
            session_id: uuid::Uuid::new_v4(),
        };
        logger.log(&event).await;
        // Проверяем, что метод не паникует
    }

    #[tokio::test]
    async fn test_tracing_audit_logger_auth_attempt() {
        let logger = TracingAuditLogger::new();
        let event = AuditEvent::AuthAttempt {
            success: false,
            ip: Some("192.168.1.1".to_string()),
        };
        logger.log(&event).await;
    }
}
