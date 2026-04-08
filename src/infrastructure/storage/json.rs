//! JSON file-based хранилище сессий.
//!
//! Каждая сессия хранится в отдельном файле `{storage_path}/{id}.json`.
//! Запись выполняется атомарно через временный файл + rename,
//! что гарантирует отсутствие повреждённых файлов при сбое.

use std::path::PathBuf;

use async_trait::async_trait;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::domain::session::{Session, SessionStore, SessionSummary};
use crate::error::{Result, SmithError};

/// Хранилище сессий в виде JSON-файлов.
///
/// # Атомарная запись
///
/// Данные сначала пишутся во временный файл `{id}.json.tmp`,
/// затем выполняется `rename(tmp, final)`. На POSIX-системах
/// rename атомарен — читатель всегда видит либо старую, либо
/// новую версию, но никогда частично записанный файл.
#[derive(Debug, Clone)]
pub struct JsonSessionStore {
    /// Базовая директория для хранения файлов сессий.
    storage_path: PathBuf,
}

impl JsonSessionStore {
    /// Создаёт хранилище с указанной директорией.
    ///
    /// # Arguments
    ///
    /// * `storage_path` — директория, где будут храниться `.json` файлы.
    ///
    /// Директория создаётся при первом вызове [`save`](Self::save).
    pub fn new(storage_path: impl Into<PathBuf>) -> Self {
        Self {
            storage_path: storage_path.into(),
        }
    }

    /// Возвращает путь к файлу сессии.
    fn session_file_path(&self, id: Uuid) -> PathBuf {
        self.storage_path.join(format!("{id}.json"))
    }

    /// Возвращает путь к временному файлу.
    fn temp_file_path(&self, id: Uuid) -> PathBuf {
        self.storage_path.join(format!("{id}.json.tmp"))
    }

    /// Обёртка для создания [`SmithError::Storage`].
    fn storage_error(operation: &str, message: impl Into<String>) -> SmithError {
        SmithError::Storage {
            operation: operation.to_string(),
            message: message.into(),
        }
    }
}

#[async_trait]
impl SessionStore for JsonSessionStore {
    async fn save(&self, session: &Session) -> Result<()> {
        // Убедимся, что директория существует
        if !self.storage_path.exists() {
            fs::create_dir_all(&self.storage_path)
                .await
                .map_err(|e| Self::storage_error("save", format!("cannot create dir: {e}")))?;
        }

        let json = serde_json::to_string_pretty(session)
            .map_err(|e| Self::storage_error("save", format!("serialization failed: {e}")))?;

        let tmp_path = self.temp_file_path(session.id);
        let final_path = self.session_file_path(session.id);

        // Пишем во временный файл, затем rename
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_path)
            .await
            .map_err(|e| Self::storage_error("save", format!("cannot open tmp file: {e}")))?;

        file.write_all(json.as_bytes())
            .await
            .map_err(|e| Self::storage_error("save", format!("write failed: {e}")))?;
        file.flush()
            .await
            .map_err(|e| Self::storage_error("save", format!("flush failed: {e}")))?;

        // Атомарный rename
        fs::rename(&tmp_path, &final_path)
            .await
            .map_err(|e| Self::storage_error("save", format!("rename failed: {e}")))?;

        debug!(id = %session.id, path = ?final_path, "session saved atomically");
        Ok(())
    }

    async fn load(&self, id: Uuid) -> Result<Option<Session>> {
        let path = self.session_file_path(id);

        if !path.exists() {
            debug!(%id, "session file not found");
            return Ok(None);
        }

        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| Self::storage_error("load", format!("read failed: {e}")))?;

        let session: Session = serde_json::from_str(&content)
            .map_err(|e| Self::storage_error("load", format!("deserialization failed: {e}")))?;

        debug!(%id, path = ?path, "session loaded");
        Ok(Some(session))
    }

    async fn list(&self) -> Result<Vec<SessionSummary>> {
        if !self.storage_path.exists() {
            return Ok(Vec::new());
        }

        let mut summaries = Vec::new();
        let mut entries = fs::read_dir(&self.storage_path)
            .await
            .map_err(|e| Self::storage_error("list", format!("read_dir failed: {e}")))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| Self::storage_error("list", format!("next_entry failed: {e}")))?
        {
            let path = entry.path();
            // Пропускаем временные и не-JSON файлы
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            let content = match fs::read_to_string(&path).await {
                Ok(c) => c,
                Err(e) => {
                    warn!(path = ?path, error = %e, "skipping unreadable session file");
                    continue;
                }
            };

            match serde_json::from_str::<Session>(&content) {
                Ok(session) => summaries.push(SessionSummary::from(&session)),
                Err(e) => {
                    warn!(path = ?path, error = %e, "skipping corrupt session file");
                }
            }
        }

        debug!(count = summaries.len(), "listed sessions");
        Ok(summaries)
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let path = self.session_file_path(id);

        if !path.exists() {
            return Ok(false);
        }

        fs::remove_file(&path)
            .await
            .map_err(|e| Self::storage_error("delete", format!("remove failed: {e}")))?;

        debug!(%id, path = ?path, "session deleted");
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message::Message;
    use tempfile::TempDir;

    fn create_test_session() -> Session {
        let id = Uuid::new_v4();
        Session::with_messages(id, vec![Message::user("hello"), Message::assistant("hi")])
    }

    fn make_store(temp_dir: &TempDir) -> JsonSessionStore {
        JsonSessionStore::new(temp_dir.path())
    }

    #[tokio::test]
    async fn test_save_and_load_round_trip() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let store = make_store(&temp_dir);
        let session = create_test_session();
        let id = session.id;

        store.save(&session).await.expect("save should succeed");

        // Файл должен существовать
        let file_path = store.session_file_path(id);
        assert!(file_path.exists());

        // Загрузка
        let loaded = store.load(id).await.expect("load should succeed");
        assert!(loaded.is_some());
        let loaded = loaded.expect("session");
        assert_eq!(loaded.id, id);
        assert_eq!(loaded.messages.len(), 2);
    }

    #[tokio::test]
    async fn test_load_nonexistent() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let store = make_store(&temp_dir);
        let id = Uuid::new_v4();

        let loaded = store.load(id).await.expect("load should succeed");
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let store = make_store(&temp_dir);

        let s1 = create_test_session();
        let s2 = create_test_session();
        store.save(&s1).await.expect("save s1");
        store.save(&s2).await.expect("save s2");

        let list = store.list().await.expect("list should succeed");
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_session() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let store = make_store(&temp_dir);
        let session = create_test_session();
        let id = session.id;

        store.save(&session).await.expect("save");
        let deleted = store.delete(id).await.expect("delete");
        assert!(deleted);

        let loaded = store.load(id).await.expect("load");
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let store = make_store(&temp_dir);
        let id = Uuid::new_v4();

        let deleted = store.delete(id).await.expect("delete");
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_list_ignores_temp_files() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let store = make_store(&temp_dir);

        // Создаём временный файл (как при сбое записи)
        let tmp_path = temp_dir.path().join("dead.json.tmp");
        tokio::fs::write(&tmp_path, "{}").await.expect("write tmp");

        let list = store.list().await.expect("list");
        assert_eq!(list.len(), 0); // .tmp файл должен игнорироваться
    }
}
