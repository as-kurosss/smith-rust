//! Менеджер сессий — use-case для загрузки/сохранения сессий.
//!
//! Оркестрирует взаимодействие между хранилищем (`SessionStore`)
//! и бизнес-логикой. Не зависит от конкретного бэкенда.

use tracing::{debug, info};
use uuid::Uuid;

use crate::domain::session::{Session, SessionStore, SessionSummary};
use crate::error::{Result, SmithError};

/// Менеджер сессий.
///
/// Хранит ссылку на активную сессию (если загружена)
/// и делегирует операции хранения в [`SessionStore`].
pub struct SessionManager<S: SessionStore> {
    /// Бэкенд хранения.
    store: S,
    /// Текущая активная сессия (если загружена).
    active_session: Option<Session>,
}

impl<S: SessionStore> SessionManager<S> {
    /// Создаёт менеджер с указанным хранилищем.
    #[must_use]
    pub fn new(store: S) -> Self {
        Self {
            store,
            active_session: None,
        }
    }

    /// Загружает сессию и делает её активной.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку при сбое хранилища или если сессия не найдена.
    pub async fn load_session(&mut self, id: Uuid) -> Result<Session> {
        info!(%id, "loading session");
        let session = self
            .store
            .load(id)
            .await?
            .ok_or_else(|| SmithError::InvalidState(format!("session {id} not found")))?;

        debug!(%id, message_count = session.messages.len(), "session loaded");
        self.active_session = Some(session.clone());
        Ok(session)
    }

    /// Сохраняет активную сессию.
    ///
    /// # Errors
    ///
    /// Возвращает [`SmithError::InvalidState`] если сессия не загружена,
    /// или ошибку хранилища при сбое записи.
    pub async fn save_active_session(&self) -> Result<()> {
        let session = self
            .active_session
            .as_ref()
            .ok_or_else(|| SmithError::InvalidState("no active session to save".to_string()))?;

        info!(id = %session.id, "saving session");
        self.store.save(session).await
    }

    /// Возвращает активную сессию (клон).
    ///
    /// # Errors
    ///
    /// Возвращает [`SmithError::InvalidState`] если сессия не загружена.
    pub fn active_session(&self) -> Result<&Session> {
        self.active_session
            .as_ref()
            .ok_or_else(|| SmithError::InvalidState("no active session".to_string()))
    }

    /// Возвращает мутабельную ссылку на активную сессию.
    ///
    /// # Errors
    ///
    /// Возвращает [`SmithError::InvalidState`] если сессия не загружена.
    pub fn active_session_mut(&mut self) -> Result<&mut Session> {
        self.active_session
            .as_mut()
            .ok_or_else(|| SmithError::InvalidState("no active session".to_string()))
    }

    /// Возвращает список всех сессий в хранилище.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку хранилища при сбое перечисления.
    pub async fn list_sessions(&self) -> Result<Vec<SessionSummary>> {
        info!("listing all sessions");
        self.store.list().await
    }

    /// Удаляет сессию из хранилища.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку хранилища при сбое удаления.
    pub async fn delete_session(&self, id: Uuid) -> Result<bool> {
        info!(%id, "deleting session");
        self.store.delete(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message::Message;
    use crate::infrastructure::storage::memory::InMemorySessionStore;

    fn create_test_session() -> Session {
        let id = Uuid::new_v4();
        Session::with_messages(id, vec![Message::user("hello"), Message::assistant("hi")])
    }

    #[tokio::test]
    async fn test_load_session() {
        let store = InMemorySessionStore::new();
        let session = create_test_session();
        let id = session.id;
        store.save(&session).await.expect("save");

        let mut manager = SessionManager::new(store);
        let loaded = manager.load_session(id).await.expect("load");
        assert_eq!(loaded.id, id);
        assert!(manager.active_session().is_ok());
    }

    #[tokio::test]
    async fn test_load_nonexistent_session() {
        let store = InMemorySessionStore::new();
        let mut manager = SessionManager::new(store);

        let result = manager.load_session(Uuid::new_v4()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_save_active_session() {
        let store = InMemorySessionStore::new();
        let mut manager = SessionManager::new(store);

        let session = create_test_session();
        manager.active_session = Some(session.clone());

        manager.save_active_session().await.expect("save");
        let loaded = manager
            .store
            .load(session.id)
            .await
            .expect("load from store");
        assert!(loaded.is_some());
    }

    #[tokio::test]
    async fn test_save_without_active_session() {
        let store = InMemorySessionStore::new();
        let manager = SessionManager::new(store);

        let result = manager.save_active_session().await;
        assert!(matches!(result, Err(SmithError::InvalidState(_))));
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let store = InMemorySessionStore::new();
        let s1 = create_test_session();
        let s2 = create_test_session();
        store.save(&s1).await.expect("save s1");
        store.save(&s2).await.expect("save s2");

        let manager = SessionManager::new(store);
        let list = manager.list_sessions().await.expect("list");
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_session() {
        let store = InMemorySessionStore::new();
        let session = create_test_session();
        let id = session.id;
        store.save(&session).await.expect("save");

        let manager = SessionManager::new(store);
        let deleted = manager.delete_session(id).await.expect("delete");
        assert!(deleted);
    }

    #[tokio::test]
    async fn test_active_session_mut() {
        let store = InMemorySessionStore::new();
        let mut manager = SessionManager::new(store);

        // Без активной сессии
        assert!(manager.active_session_mut().is_err());

        // С активной сессией
        manager.active_session = Some(create_test_session());
        let session = manager.active_session_mut().expect("active");
        session.add_message(Message::user("new message"));
        assert_eq!(session.messages.len(), 3);
    }
}
