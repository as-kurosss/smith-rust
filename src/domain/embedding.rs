//! Трейт для провайдеров эмбеддингов (векторных представлений текста).

use async_trait::async_trait;

use crate::error::Result;

/// Провайдер эмбеддингов.
///
/// Преобразует текст в вектор фиксированной размерности.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Генерирует эмбеддинг для текста.
    ///
    /// # Arguments
    ///
    /// * `text` — текст для векторизации.
    ///
    /// # Errors
    ///
    /// Возвращает [`crate::error::SmithError::Memory`] при сбое запроса
    /// или несовпадении размерности.
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Возвращает размерность эмбеддингов (например, 1536 для text-embedding-3-small).
    fn dimension(&self) -> usize;
}
