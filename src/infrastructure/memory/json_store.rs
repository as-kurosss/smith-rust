//! JSON-based хранилище фрагментов памяти.
//!
//! Все чанки хранятся в одном файле `{memory_path}/memory.json`.
//! Атомарная запись: write temp → rename.
//! Поиск: cosine similarity + фильтрация по тегам.

use std::path::PathBuf;

use async_trait::async_trait;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use tracing::{debug, warn};

use crate::domain::memory::cosine_similarity;
use crate::domain::memory::{MemoryChunk, MemoryStore};
use crate::error::{Result, SmithError};

/// JSON-хранилище фрагментов памяти.
#[derive(Debug, Clone)]
pub struct JsonMemoryStore {
    /// Путь к файлу memory.json.
    file_path: PathBuf,
}

impl JsonMemoryStore {
    /// Создаёт хранилище с указанной директорией.
    ///
    /// Файл будет `{storage_path}/memory.json`.
    #[must_use]
    pub fn new(storage_path: impl Into<PathBuf>) -> Self {
        let path = storage_path.into().join("memory.json");
        Self { file_path: path }
    }

    /// Загружает все чанки из файла.
    async fn load_chunks(&self) -> Result<Vec<MemoryChunk>> {
        if !self.file_path.exists() {
            return Ok(Vec::new());
        }
        let content =
            fs::read_to_string(&self.file_path)
                .await
                .map_err(|e| SmithError::Memory {
                    operation: "search".to_string(),
                    message: format!("read failed: {e}"),
                })?;
        let chunks: Vec<MemoryChunk> =
            serde_json::from_str(&content).map_err(|e| SmithError::Memory {
                operation: "search".to_string(),
                message: format!("deserialize failed: {e}"),
            })?;
        Ok(chunks)
    }

    /// Сохраняет все чанки атомарно.
    async fn save_chunks(&self, chunks: &[MemoryChunk]) -> Result<()> {
        if let Some(parent) = self.file_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| SmithError::Memory {
                        operation: "add".to_string(),
                        message: format!("create dir failed: {e}"),
                    })?;
            }
        }

        let json = serde_json::to_string_pretty(chunks).map_err(|e| SmithError::Memory {
            operation: "add".to_string(),
            message: format!("serialize failed: {e}"),
        })?;

        let tmp_path = self.file_path.with_extension("json.tmp");
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_path)
            .await
            .map_err(|e| SmithError::Memory {
                operation: "add".to_string(),
                message: format!("open tmp failed: {e}"),
            })?;

        file.write_all(json.as_bytes())
            .await
            .map_err(|e| SmithError::Memory {
                operation: "add".to_string(),
                message: format!("write failed: {e}"),
            })?;
        file.flush().await.map_err(|e| SmithError::Memory {
            operation: "add".to_string(),
            message: format!("flush failed: {e}"),
        })?;

        fs::rename(&tmp_path, &self.file_path)
            .await
            .map_err(|e| SmithError::Memory {
                operation: "add".to_string(),
                message: format!("rename failed: {e}"),
            })?;

        debug!(path = ?self.file_path, chunk_count = chunks.len(), "memory saved atomically");
        Ok(())
    }
}

#[async_trait]
impl MemoryStore for JsonMemoryStore {
    async fn add_chunk(&self, chunk: MemoryChunk) -> Result<()> {
        let mut chunks = self.load_chunks().await?;
        chunks.push(chunk);
        self.save_chunks(&chunks).await
    }

    async fn get_chunk(&self, id: &str) -> Result<Option<MemoryChunk>> {
        let chunks = self.load_chunks().await?;
        Ok(chunks.into_iter().find(|c| c.id == id))
    }

    async fn search(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<MemoryChunk>> {
        let chunks = self.load_chunks().await?;
        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        // Вычисляем similarity для каждого чанка
        let mut scored: Vec<(f32, MemoryChunk)> = chunks
            .into_iter()
            .filter_map(|chunk| {
                if chunk.embedding.len() != query_embedding.len() {
                    warn!(
                        chunk_id = %chunk.id,
                        chunk_dim = chunk.embedding.len(),
                        query_dim = query_embedding.len(),
                        "dimension mismatch, skipping chunk"
                    );
                    return None;
                }
                let sim = cosine_similarity(&chunk.embedding, query_embedding);
                Some((sim, chunk))
            })
            .collect();

        // Сортируем по убыванию similarity
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Берём top-k
        scored.truncate(top_k);
        let results: Vec<MemoryChunk> = scored.into_iter().map(|(_, c)| c).collect();

        debug!(
            result_count = results.len(),
            top_k, "memory search completed"
        );
        Ok(results)
    }

    async fn clear(&self) -> Result<()> {
        if self.file_path.exists() {
            fs::remove_file(&self.file_path)
                .await
                .map_err(|e| SmithError::Memory {
                    operation: "clear".to_string(),
                    message: format!("remove failed: {e}"),
                })?;
        }
        debug!(path = ?self.file_path, "memory cleared");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    use crate::domain::memory::ChunkMetadata;

    fn make_store(temp_dir: &TempDir) -> JsonMemoryStore {
        JsonMemoryStore::new(temp_dir.path())
    }

    fn make_chunk(content: &str, embedding: Vec<f32>) -> MemoryChunk {
        MemoryChunk::new(content, embedding, ChunkMetadata::new("user_message"))
    }

    #[tokio::test]
    async fn test_add_and_search() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let store = make_store(&temp_dir);

        store
            .add_chunk(make_chunk("hello world", vec![1.0, 0.0, 0.0]))
            .await
            .expect("add chunk");
        store
            .add_chunk(make_chunk("goodbye world", vec![0.0, 1.0, 0.0]))
            .await
            .expect("add chunk");

        // Поиск по похожему вектору
        let results = store.search(&[1.0, 0.0, 0.0], 1).await.expect("search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "hello world");
    }

    #[tokio::test]
    async fn test_get_chunk() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let store = make_store(&temp_dir);
        let chunk = make_chunk("test", vec![1.0]);
        let id = chunk.id.clone();

        store.add_chunk(chunk).await.expect("add");
        let loaded = store.get_chunk(&id).await.expect("get");
        assert!(loaded.is_some());
        assert_eq!(loaded.expect("chunk").content, "test");
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let store = make_store(&temp_dir);
        let loaded = store.get_chunk("nonexistent").await.expect("get");
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_clear() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let store = make_store(&temp_dir);
        store
            .add_chunk(make_chunk("test", vec![1.0]))
            .await
            .expect("add");
        store.clear().await.expect("clear");
        let results = store.search(&[1.0], 10).await.expect("search");
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_search_empty_store() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let store = make_store(&temp_dir);
        let results = store.search(&[1.0], 5).await.expect("search");
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_search_dimension_mismatch() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let store = make_store(&temp_dir);
        store
            .add_chunk(make_chunk("test", vec![1.0, 0.0]))
            .await
            .expect("add");

        // Запрос с другой размерностью
        let results = store.search(&[1.0], 5).await.expect("search");
        assert!(results.is_empty()); // chunk пропущен из-за mismatch
    }

    #[tokio::test]
    async fn test_top_k_order() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let store = make_store(&temp_dir);

        store
            .add_chunk(make_chunk("alpha", vec![1.0, 0.0, 0.0]))
            .await
            .expect("add");
        store
            .add_chunk(make_chunk("beta", vec![0.8, 0.6, 0.0]))
            .await
            .expect("add");
        store
            .add_chunk(make_chunk("gamma", vec![0.0, 1.0, 0.0]))
            .await
            .expect("add");

        let results = store.search(&[1.0, 0.0, 0.0], 3).await.expect("search");
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].content, "alpha");
        assert_eq!(results[1].content, "beta");
        assert_eq!(results[2].content, "gamma");
    }
}
