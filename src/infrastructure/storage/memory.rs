//! In-memory хранилище сессий на базе `DashMap`.
//!
//! Предназначено исключительно для тестирования.
//! Не выполняет реального I/O — все операции атомарны в памяти.

use async_trait::async_trait;
use dashmap::DashMap;
use tracing::debug;
use uuid::Uuid;

use crate::domain::session::{Session, SessionStore, SessionSummary};
use crate::error::Result;

/// In-memory хранилище сессий.
///
/// Использует `DashMap` для обеспечения параллельного доступа
/// без глобальной блокировки.
#[derive(Debug)]
pub struct InMemorySessionStore {
    /// Хранилище: Uuid → Session.
    sessions: DashMap<Uuid, Session>,
}

impl InMemorySessionStore {
    /// Создаёт пустое хранилище.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }
}

impl Default for InMemorySessionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn save(&self, session: &Session) -> Result<()> {
        debug!(id = %session.id, message_count = session.messages.len(), "saving session to memory");
        self.sessions.insert(session.id, session.clone());
        Ok(())
    }

    async fn load(&self, id: Uuid) -> Result<Option<Session>> {
        debug!(%id, "loading session from memory");
        Ok(self.sessions.get(&id).map(|entry| entry.value().clone()))
    }

    async fn list(&self) -> Result<Vec<SessionSummary>> {
        debug!(count = self.sessions.len(), "listing all sessions");
        Ok(self
            .sessions
            .iter()
            .map(|entry| SessionSummary::from(entry.value()))
            .collect())
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        debug!(%id, "deleting session from memory");
        Ok(self.sessions.remove(&id).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message::Message;

    fn create_test_session() -> Session {
        let id = Uuid::new_v4();
        Session::with_messages(id, vec![Message::user("test"), Message::assistant("reply")])
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let store = InMemorySessionStore::new();
        let session = create_test_session();
        let id = session.id;

        store.save(&session).await.expect("save should succeed");
        let loaded = store.load(id).await.expect("load should succeed");

        assert!(loaded.is_some());
        let loaded = loaded.expect("session exists");
        assert_eq!(loaded.id, id);
        assert_eq!(loaded.messages.len(), 2);
    }

    #[tokio::test]
    async fn test_load_nonexistent() {
        let store = InMemorySessionStore::new();
        let id = Uuid::new_v4();

        let loaded = store.load(id).await.expect("load should succeed");
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_list() {
        let store = InMemorySessionStore::new();
        let s1 = create_test_session();
        let s2 = create_test_session();

        store.save(&s1).await.expect("save s1");
        store.save(&s2).await.expect("save s2");

        let list = store.list().await.expect("list should succeed");
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_delete() {
        let store = InMemorySessionStore::new();
        let session = create_test_session();
        let id = session.id;

        store.save(&session).await.expect("save should succeed");

        let deleted = store.delete(id).await.expect("delete should succeed");
        assert!(deleted);

        let loaded = store.load(id).await.expect("load should succeed");
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let store = InMemorySessionStore::new();
        let id = Uuid::new_v4();

        let deleted = store.delete(id).await.expect("delete should succeed");
        assert!(!deleted);
    }
}
