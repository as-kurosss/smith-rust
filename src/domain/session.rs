//! Доменные типы сессий и трейт хранилища.
//!
//! Этот модуль определяет контракт для всех бэкендов хранения
//! (JSON, PostgreSQL, in-memory) без привязки к конкретной реализации.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::message::Message;
use crate::error::Result;

/// Метаданные сессии (не включают сами сообщения).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Опциональное название сессии.
    pub title: Option<String>,
    /// Количество сообщений в сессии.
    pub message_count: usize,
    /// Опциональные пользовательские метки.
    pub tags: Vec<String>,
}

impl SessionMetadata {
    /// Создаёт пустые метаданные.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            title: None,
            message_count: 0,
            tags: Vec::new(),
        }
    }
}

impl Default for SessionMetadata {
    fn default() -> Self {
        Self::empty()
    }
}

/// Полная сессия с историей сообщений.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Уникальный идентификатор.
    pub id: Uuid,
    /// Время создания.
    pub created_at: DateTime<Utc>,
    /// Время последнего обновления.
    pub updated_at: DateTime<Utc>,
    /// История сообщений.
    pub messages: Vec<Message>,
    /// Метаданные.
    pub metadata: SessionMetadata,
}

impl Session {
    /// Создаёт новую пустую сессию.
    #[must_use]
    pub fn new(id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id,
            created_at: now,
            updated_at: now,
            messages: Vec::new(),
            metadata: SessionMetadata::empty(),
        }
    }

    /// Создаёт сессию из существующей истории сообщений.
    #[must_use]
    pub fn with_messages(id: Uuid, messages: Vec<Message>) -> Self {
        let now = Utc::now();
        let count = messages.len();
        Self {
            id,
            created_at: now,
            updated_at: now,
            messages,
            metadata: SessionMetadata {
                message_count: count,
                ..SessionMetadata::empty()
            },
        }
    }

    /// Добавляет сообщение и обновляет timestamp.
    pub fn add_message(&mut self, msg: Message) {
        self.messages.push(msg);
        self.updated_at = Utc::now();
        self.metadata.message_count = self.messages.len();
    }
}

/// Краткая информация о сессии (для вывода списка).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// Идентификатор сессии.
    pub id: Uuid,
    /// Время создания.
    pub created_at: DateTime<Utc>,
    /// Время последнего обновления.
    pub updated_at: DateTime<Utc>,
    /// Название (если есть).
    pub title: Option<String>,
    /// Количество сообщений.
    pub message_count: usize,
}

impl From<&Session> for SessionSummary {
    fn from(session: &Session) -> Self {
        Self {
            id: session.id,
            created_at: session.created_at,
            updated_at: session.updated_at,
            title: session.metadata.title.clone(),
            message_count: session.metadata.message_count,
        }
    }
}

/// Трейт для хранилищ сессий.
///
/// Все реализации должны быть `Send + Sync` для использования
/// в асинхронном контексте.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Сохраняет сессию.
    ///
    /// # Errors
    ///
    /// Возвращает [`SmithError::Storage`] при сбое записи.
    async fn save(&self, session: &Session) -> Result<()>;

    /// Загружает сессию по идентификатору.
    ///
    /// Возвращает `None` если сессия не найдена (не ошибка).
    ///
    /// # Errors
    ///
    /// Возвращает [`SmithError::Storage`] при сбое чтения.
    async fn load(&self, id: Uuid) -> Result<Option<Session>>;

    /// Возвращает краткий список всех сохранённых сессий.
    ///
    /// # Errors
    ///
    /// Возвращает [`SmithError::Storage`] при сбое перечисления.
    async fn list(&self) -> Result<Vec<SessionSummary>>;

    /// Удаляет сессию. Возвращает `true` если была удалена.
    ///
    /// # Errors
    ///
    /// Возвращает [`SmithError::Storage`] при сбое удаления.
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let id = Uuid::new_v4();
        let session = Session::new(id);
        assert_eq!(session.id, id);
        assert!(session.messages.is_empty());
        assert_eq!(session.metadata.message_count, 0);
    }

    #[test]
    fn test_session_with_messages() {
        let id = Uuid::new_v4();
        let messages = vec![Message::user("hello"), Message::assistant("hi")];
        let session = Session::with_messages(id, messages.clone());
        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.metadata.message_count, 2);
    }

    #[test]
    fn test_session_add_message() {
        let id = Uuid::new_v4();
        let mut session = Session::new(id);
        session.add_message(Message::user("test"));
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.metadata.message_count, 1);
    }

    #[test]
    fn test_session_summary_from_session() {
        let id = Uuid::new_v4();
        let session = Session::new(id);
        let summary = SessionSummary::from(&session);
        assert_eq!(summary.id, id);
        assert_eq!(summary.message_count, 0);
        assert!(summary.title.is_none());
    }
}
