//! Менеджер контекста и памяти — координация chunking, embedding и background processing.
//!
//! Автоматическое наполнение памяти через bounded mpsc канал:
//! - `chat_loop` отправляет сообщения в канал (неблокирующе).
//! - Фоновая задача генерирует эмбеддинги и сохраняет чанки.

use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::domain::embedding::EmbeddingProvider;
use crate::domain::memory::{ChunkMetadata, MemoryChunk, MemoryStore};

/// Размер чанка для разбиения длинных текстов.
pub const CHUNK_SIZE: usize = 500;
/// Перекрытие между соседними чанками.
pub const CHUNK_OVERLAP: usize = 50;

/// Сообщение для фоновой задачи обработки памяти.
pub struct IngestTask {
    content: String,
    metadata: ChunkMetadata,
}

/// Менеджер контекста и памяти.
pub struct ContextManager {
    /// Отправитель для фоновой задачи.
    sender: mpsc::Sender<IngestTask>,
}

impl ContextManager {
    /// Создаёт менеджер без фонового воркера.
    ///
    /// Воркер должен быть запущен вызывающим кодом (обычно в `main.rs`)
    /// через [`spawn_background_worker`](Self::spawn_background_worker).
    #[must_use]
    pub fn new(channel_capacity: usize) -> Self {
        let (tx, _rx) = mpsc::channel::<IngestTask>(channel_capacity);
        Self { sender: tx }
    }

    /// Создаёт канал и возвращает обе стороны (sender + receiver).
    ///
    /// Receiver должен быть передан фоновой задаче.
    #[must_use]
    pub fn with_channel(channel_capacity: usize) -> (Self, mpsc::Receiver<IngestTask>) {
        let (tx, rx) = mpsc::channel::<IngestTask>(channel_capacity);
        (Self { sender: tx }, rx)
    }

    /// Запускает фоновую задачу обработки памяти.
    ///
    /// # Arguments
    ///
    /// * `rx` — приёмник задач (полученный из `with_channel`).
    /// * `store` — хранилище памяти.
    /// * `embedding_provider` — провайдер эмбеддингов.
    ///
    /// Возвращает JoinHandle фоновой задачи.
    pub fn spawn_background_worker(
        mut rx: mpsc::Receiver<IngestTask>,
        store: Arc<dyn MemoryStore>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!("memory background worker started");
            while let Some(task) = rx.recv().await {
                // Разбиваем на чанки
                let chunks = chunk_text(&task.content, &task.metadata);

                for chunk_text in chunks {
                    match embedding_provider.embed(&chunk_text).await {
                        Ok(embedding) => {
                            let chunk =
                                MemoryChunk::new(chunk_text, embedding, task.metadata.clone());
                            if let Err(e) = store.add_chunk(chunk).await {
                                warn!(error = %e, "failed to add chunk to memory store");
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, "failed to generate embedding");
                        }
                    }
                }
            }
            info!("memory background worker stopped");
        })
    }

    /// Добавляет сообщение в очередь на обработку (неблокирующе).
    ///
    /// Если канал полон, сообщение отбрасывается с предупреждением.
    pub fn ingest_message(&self, content: String, source: String) {
        let task = IngestTask {
            content,
            metadata: ChunkMetadata::new(source),
        };

        // try_send не блокирует — если канал полон, отбрасываем
        if let Err(e) = self.sender.try_send(task) {
            match e {
                mpsc::error::TrySendError::Full(_) => {
                    warn!("memory ingest channel is full, dropping message");
                }
                mpsc::error::TrySendError::Closed(_) => {
                    error!("memory ingest channel is closed");
                }
            }
        }
    }

    /// Ручное добавление чанка (синхронно, bypass channel).
    ///
    /// # Errors
    ///
    /// Возвращает ошибку при сбое хранилища.
    pub async fn add_chunk(
        &self,
        chunk: MemoryChunk,
        store: &dyn MemoryStore,
    ) -> crate::error::Result<()> {
        store.add_chunk(chunk).await
    }

    /// Очищает всю память.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку при сбое хранилища.
    pub async fn clear(&self, store: &dyn MemoryStore) -> crate::error::Result<()> {
        store.clear().await
    }
}

/// Разбивает текст на перекрывающиеся чанки.
///
/// Если текст короче `CHUNK_SIZE`, возвращается один чанк.
#[must_use]
pub fn chunk_text(text: &str, _metadata: &ChunkMetadata) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= CHUNK_SIZE {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let step = CHUNK_SIZE.saturating_sub(CHUNK_OVERLAP);
    let mut start = 0;

    while start < chars.len() {
        let end = (start + CHUNK_SIZE).min(chars.len());
        let chunk: String = chars[start..end].iter().collect();
        chunks.push(chunk);

        if end >= chars.len() {
            break;
        }
        start += step;
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use crate::domain::memory::MemoryStore;

    #[derive(Debug, Default)]
    struct MockStore {
        chunks: Arc<Mutex<Vec<MemoryChunk>>>,
    }

    #[async_trait::async_trait]
    impl MemoryStore for MockStore {
        async fn add_chunk(&self, chunk: MemoryChunk) -> crate::error::Result<()> {
            self.chunks.lock().await.push(chunk);
            Ok(())
        }

        async fn get_chunk(&self, _id: &str) -> crate::error::Result<Option<MemoryChunk>> {
            Ok(None)
        }

        async fn search(
            &self,
            _query_embedding: &[f32],
            _top_k: usize,
        ) -> crate::error::Result<Vec<MemoryChunk>> {
            Ok(Vec::new())
        }

        async fn clear(&self) -> crate::error::Result<()> {
            self.chunks.lock().await.clear();
            Ok(())
        }
    }

    #[derive(Debug)]
    struct MockEmbeddingProvider;

    #[async_trait::async_trait]
    impl crate::domain::embedding::EmbeddingProvider for MockEmbeddingProvider {
        async fn embed(&self, _text: &str) -> crate::error::Result<Vec<f32>> {
            Ok(vec![0.0; 3])
        }

        fn dimension(&self) -> usize {
            3
        }
    }

    #[tokio::test]
    async fn test_chunk_text_short() {
        let meta = ChunkMetadata::new("test");
        let chunks = chunk_text("hello world", &meta);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "hello world");
    }

    #[tokio::test]
    async fn test_chunk_text_long() {
        let meta = ChunkMetadata::new("test");
        let text: String = (0..1000).map(|_| 'a').collect();
        let chunks = chunk_text(&text, &meta);
        assert!(chunks.len() > 1);
        // Первый чанк = CHUNK_SIZE символов
        assert_eq!(chunks[0].len(), CHUNK_SIZE);
    }

    #[tokio::test]
    async fn test_context_manager_ingest() {
        let store = Arc::new(MockStore::default());
        let provider = Arc::new(MockEmbeddingProvider);
        let (manager, rx) = ContextManager::with_channel(32);

        // Запускаем воркер вручную
        let worker_handle = ContextManager::spawn_background_worker(rx, store.clone(), provider);

        manager.ingest_message("test message".to_string(), "user_message".to_string());

        // Даём время фоновой задаче обработать
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        drop(manager); // закрываем канал
        worker_handle.abort();

        let chunks = store.chunks.lock().await;
        assert!(!chunks.is_empty());
    }
}
