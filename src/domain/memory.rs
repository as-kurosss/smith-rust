//! Типы и трейт для долгосрочной памяти (memory store).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;

/// Метаданные фрагмента памяти.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Время создания.
    pub created_at: DateTime<Utc>,
    /// Источник: `user_message`, `assistant_message`, `system_prompt`, `manual`.
    pub source: String,
    /// Пользовательские теги.
    pub tags: Vec<String>,
}

impl ChunkMetadata {
    /// Создаёт метаданные с указанным источником.
    #[must_use]
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            created_at: Utc::now(),
            source: source.into(),
            tags: Vec::new(),
        }
    }

    /// Добавляет теги.
    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

/// Фрагмент памяти с векторным представлением.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryChunk {
    /// Уникальный идентификатор.
    pub id: String,
    /// Текстовое содержимое.
    pub content: String,
    /// Вектор эмбеддинга (размерность зависит от провайдера).
    pub embedding: Vec<f32>,
    /// Метаданные.
    pub metadata: ChunkMetadata,
}

impl MemoryChunk {
    /// Создаёт новый фрагмент.
    #[must_use]
    pub fn new(content: impl Into<String>, embedding: Vec<f32>, metadata: ChunkMetadata) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: content.into(),
            embedding,
            metadata,
        }
    }
}

/// Трейт хранилища памяти.
///
/// Все реализации должны быть `Send + Sync` для использования
/// в асинхронном контексте.
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Добавляет фрагмент в хранилище.
    ///
    /// # Errors
    ///
    /// Возвращает [`crate::error::SmithError::Memory`] при сбое записи.
    async fn add_chunk(&self, chunk: MemoryChunk) -> Result<()>;

    /// Получает фрагмент по идентификатору.
    ///
    /// # Errors
    ///
    /// Возвращает [`crate::error::SmithError::Memory`] при сбое чтения.
    async fn get_chunk(&self, id: &str) -> Result<Option<MemoryChunk>>;

    /// Поиск top-K наиболее релевантных фрагментов по cosine similarity.
    ///
    /// # Errors
    ///
    /// Возвращает [`crate::error::SmithError::Memory`] при сбое поиска.
    async fn search(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<MemoryChunk>>;

    /// Очищает все фрагменты.
    ///
    /// # Errors
    ///
    /// Возвращает [`crate::error::SmithError::Memory`] при сбое удаления.
    async fn clear(&self) -> Result<()>;
}

/// Вычисляет cosine similarity между двумя векторами.
///
/// Формула: `dot(a, b) / (norm(a) * norm(b) + 1e-9)`
///
/// # Panics
///
/// Паникует при несовпадении размерностей векторов.
#[must_use]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "vector dimensions must match");

    let dot = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum::<f32>();
    let norm_a = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    dot / (norm_a * norm_b + 1e-9)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_parallel_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![2.0, 4.0, 6.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_orthogonal_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-5);
    }

    #[test]
    fn test_cosine_opposite_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_identical_vectors() {
        let a = vec![3.0, 4.0];
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_chunk_metadata_creation() {
        let meta = ChunkMetadata::new("user_message");
        assert_eq!(meta.source, "user_message");
        assert!(meta.tags.is_empty());
    }

    #[test]
    fn test_chunk_metadata_with_tags() {
        let meta = ChunkMetadata::new("test").with_tags(vec!["tag1".to_string()]);
        assert_eq!(meta.tags.len(), 1);
    }

    #[test]
    fn test_memory_chunk_creation() {
        let meta = ChunkMetadata::new("test");
        let chunk = MemoryChunk::new("hello world", vec![0.1, 0.2], meta);
        assert!(!chunk.id.is_empty());
        assert_eq!(chunk.content, "hello world");
        assert_eq!(chunk.embedding.len(), 2);
    }
}
