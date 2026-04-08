//! PostgreSQL хранилище сессий.
//!
//! Использует `sqlx` с компайл-тайм проверкой запросов.
//!
//! # Схема таблицы
//!
//! ```sql
//! CREATE TABLE sessions (
//!     id UUID PRIMARY KEY,
//!     created_at TIMESTAMPTZ NOT NULL,
//!     updated_at TIMESTAMPTZ NOT NULL,
//!     messages JSONB NOT NULL DEFAULT '[]',
//!     metadata JSONB NOT NULL DEFAULT '{}'
//! );
//! ```

use async_trait::async_trait;
use sqlx::{PgPool, Row};
use tracing::debug;
use uuid::Uuid;

use crate::domain::message::Message;
use crate::domain::session::{Session, SessionMetadata, SessionStore, SessionSummary};
use crate::error::{Result, SmithError};

/// PostgreSQL хранилище сессий.
#[derive(Debug, Clone)]
pub struct PgSessionStore {
    /// Пул соединений.
    pool: PgPool,
}

impl PgSessionStore {
    /// Создаёт хранилище с существующим пулом.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Создаёт таблицу `sessions` если она не существует.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку при сбое выполнения DDL.
    pub async fn init_table(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id UUID PRIMARY KEY,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                messages JSONB NOT NULL DEFAULT '[]',
                metadata JSONB NOT NULL DEFAULT '{}'
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| SmithError::Storage {
            operation: "init".to_string(),
            message: format!("DDL failed: {e}"),
        })?;

        debug!("sessions table ensured");
        Ok(())
    }

    fn storage_error(operation: &str, message: impl Into<String>) -> SmithError {
        SmithError::Storage {
            operation: operation.to_string(),
            message: message.into(),
        }
    }
}

#[async_trait]
impl SessionStore for PgSessionStore {
    async fn save(&self, session: &Session) -> Result<()> {
        let messages_json = serde_json::to_value(&session.messages)
            .map_err(|e| Self::storage_error("save", format!("serialize messages: {e}")))?;

        let metadata_json = serde_json::to_value(&session.metadata)
            .map_err(|e| Self::storage_error("save", format!("serialize metadata: {e}")))?;

        sqlx::query(
            r#"
            INSERT INTO sessions (id, created_at, updated_at, messages, metadata)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE SET
                updated_at = EXCLUDED.updated_at,
                messages = EXCLUDED.messages,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(session.id)
        .bind(session.created_at)
        .bind(session.updated_at)
        .bind(messages_json)
        .bind(metadata_json)
        .execute(&self.pool)
        .await
        .map_err(|e| Self::storage_error("save", format!("SQL error: {e}")))?;

        debug!(id = %session.id, "session saved to postgres");
        Ok(())
    }

    async fn load(&self, id: Uuid) -> Result<Option<Session>> {
        let row = sqlx::query(
            r#"
            SELECT id, created_at, updated_at, messages, metadata
            FROM sessions WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Self::storage_error("load", format!("SQL error: {e}")))?;

        let Some(row) = row else {
            debug!(%id, "session not found");
            return Ok(None);
        };

        let messages_json: serde_json::Value = row
            .try_get("messages")
            .map_err(|e| Self::storage_error("load", format!("read messages json: {e}")))?;
        let messages: Vec<Message> = serde_json::from_value(messages_json)
            .map_err(|e| Self::storage_error("load", format!("deserialize messages: {e}")))?;

        let metadata_json: serde_json::Value = row
            .try_get("metadata")
            .map_err(|e| Self::storage_error("load", format!("read metadata json: {e}")))?;
        let metadata: SessionMetadata = serde_json::from_value(metadata_json)
            .map_err(|e| Self::storage_error("load", format!("deserialize metadata: {e}")))?;

        Ok(Some(Session {
            id: row
                .try_get("id")
                .map_err(|e| Self::storage_error("load", format!("read id: {e}")))?,
            created_at: row
                .try_get("created_at")
                .map_err(|e| Self::storage_error("load", format!("read created_at: {e}")))?,
            updated_at: row
                .try_get("updated_at")
                .map_err(|e| Self::storage_error("load", format!("read updated_at: {e}")))?,
            messages,
            metadata,
        }))
    }

    async fn list(&self) -> Result<Vec<SessionSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT id, created_at, updated_at, metadata
            FROM sessions ORDER BY updated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Self::storage_error("list", format!("SQL error: {e}")))?;

        let mut summaries = Vec::with_capacity(rows.len());
        for row in rows {
            let metadata_json: serde_json::Value = row
                .try_get("metadata")
                .map_err(|e| Self::storage_error("list", format!("read metadata json: {e}")))?;
            let metadata: SessionMetadata = serde_json::from_value(metadata_json)
                .map_err(|e| Self::storage_error("list", format!("deserialize metadata: {e}")))?;

            summaries.push(SessionSummary {
                id: row
                    .try_get("id")
                    .map_err(|e| Self::storage_error("list", format!("read id: {e}")))?,
                created_at: row
                    .try_get("created_at")
                    .map_err(|e| Self::storage_error("list", format!("read created_at: {e}")))?,
                updated_at: row
                    .try_get("updated_at")
                    .map_err(|e| Self::storage_error("list", format!("read updated_at: {e}")))?,
                title: metadata.title,
                message_count: metadata.message_count,
            });
        }

        debug!(count = summaries.len(), "listed sessions from postgres");
        Ok(summaries)
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Self::storage_error("delete", format!("SQL error: {e}")))?;

        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // PostgreSQL тест требует запущенный сервер.
    // Skip по умолчанию — запускается явно с флагом --ignored.
    //
    // Пример запуска:
    // DATABASE_URL=postgres://user:pass@localhost/smith cargo test --features postgres -- --ignored

    #[tokio::test]
    #[ignore = "requires running PostgreSQL server"]
    async fn test_postgres_roundtrip() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for postgres tests");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("connect to postgres");

        sqlx::query("DROP TABLE IF EXISTS sessions")
            .execute(&pool)
            .await
            .expect("drop table");

        let store = PgSessionStore::new(pool);
        store.init_table().await.expect("init table");

        let session = Session::new(Uuid::new_v4());
        store.save(&session).await.expect("save");

        let loaded = store.load(session.id).await.expect("load");
        assert!(loaded.is_some());
        let loaded = loaded.expect("session");
        assert_eq!(loaded.id, session.id);

        store.delete(session.id).await.expect("delete");
        let loaded = store.load(session.id).await.expect("load");
        assert!(loaded.is_none());
    }
}
