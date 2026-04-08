//! MemorySearchTool — инструмент поиска релевантных фрагментов в памяти.
//!
//! LLM вызывает этот инструмент для получения контекста из долгосрочной памяти.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use crate::domain::embedding::EmbeddingProvider;
use crate::domain::memory::MemoryStore;
use crate::domain::tool::{Tool, ToolOutput};
use crate::error::{Result, SmithError};

/// Инструмент для поиска релевантных фрагментов памяти.
pub struct MemorySearchTool {
    store: Arc<dyn MemoryStore>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    /// По умолчанию: 3.
    default_top_k: usize,
}

impl MemorySearchTool {
    /// Создаёт новый экземпляр.
    #[must_use]
    pub fn new(
        store: Arc<dyn MemoryStore>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        default_top_k: usize,
    ) -> Self {
        Self {
            store,
            embedding_provider,
            default_top_k,
        }
    }
}

#[async_trait]
impl Tool for MemorySearchTool {
    fn name(&self) -> &str {
        "memory_search"
    }

    fn description(&self) -> &str {
        "Searches long-term memory for relevant context. Returns top-k relevant memory chunks based on semantic similarity."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query to find relevant memory chunks."
                },
                "limit": {
                    "type": "number",
                    "description": "Maximum number of results to return. Default: 3."
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolOutput> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SmithError::ToolExecution {
                tool_name: self.name().to_string(),
                message: "missing or invalid 'query' parameter".to_string(),
            })?;

        let top_k = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.default_top_k as u64) as usize;

        // Генерируем эмбеддинг запроса
        let query_embedding = self.embedding_provider.embed(query).await?;

        // Ищем релевантные чанки
        let chunks = self.store.search(&query_embedding, top_k).await?;

        if chunks.is_empty() {
            return Ok(ToolOutput::success("No relevant memory chunks found."));
        }

        // Форматируем результаты
        let results: Vec<String> = chunks
            .iter()
            .enumerate()
            .map(|(i, c)| format!("[{}] {}\n(source: {})", i + 1, c.content, c.metadata.source))
            .collect();

        Ok(ToolOutput::success(results.join("\n\n")))
    }
}
