//! События чата для обновления TUI.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Событие для обновления TUI.
#[derive(Debug, Clone)]
pub enum ChatEvent {
    /// Пользователь отправил сообщение.
    UserMessage {
        session_id: Uuid,
        content: String,
        timestamp: DateTime<Utc>,
    },
    /// Ассистент ответил.
    AssistantMessage {
        session_id: Uuid,
        content: String,
        timestamp: DateTime<Utc>,
    },
    /// Начат вызов инструмента.
    ToolCall {
        session_id: Uuid,
        tool_name: String,
        arguments: String,
        timestamp: DateTime<Utc>,
    },
    /// Инструмент завершил выполнение.
    ToolResult {
        session_id: Uuid,
        tool_name: String,
        content: String,
        success: bool,
        timestamp: DateTime<Utc>,
    },
    /// Ошибка при обработке.
    Error {
        session_id: Uuid,
        message: String,
        timestamp: DateTime<Utc>,
    },
    /// Статус "думаю..."
    Thinking { session_id: Uuid, thinking: bool },
}

impl ChatEvent {
    /// Возвращает session_id события.
    #[must_use]
    pub fn session_id(&self) -> Uuid {
        match self {
            Self::UserMessage { session_id, .. } => *session_id,
            Self::AssistantMessage { session_id, .. } => *session_id,
            Self::ToolCall { session_id, .. } => *session_id,
            Self::ToolResult { session_id, .. } => *session_id,
            Self::Error { session_id, .. } => *session_id,
            Self::Thinking { session_id, .. } => *session_id,
        }
    }

    /// Возвращает временную метку.
    #[must_use]
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::UserMessage { timestamp, .. } => *timestamp,
            Self::AssistantMessage { timestamp, .. } => *timestamp,
            Self::ToolCall { timestamp, .. } => *timestamp,
            Self::ToolResult { timestamp, .. } => *timestamp,
            Self::Error { timestamp, .. } => *timestamp,
            Self::Thinking { .. } => Utc::now(),
        }
    }
}
