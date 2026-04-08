//! OpenAI Embedding Provider.
//!
//! Использует `POST /v1/embeddings` с моделью `text-embedding-3-small` (1536 dim).

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::domain::embedding::EmbeddingProvider;
use crate::error::{Result, SmithError};

/// Стандартная размерность text-embedding-3-small.
pub const DEFAULT_EMBEDDING_DIM: usize = 1536;

/// Провайдер эмбеддингов через OpenAI API.
#[derive(Debug)]
pub struct OpenAIEmbeddingProvider {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
    dimension: usize,
}

impl OpenAIEmbeddingProvider {
    /// Создаёт новый экземпляр.
    ///
    /// # Arguments
    ///
    /// * `base_url` — базовый URL (например, `https://api.openai.com`).
    /// * `api_key` — ключ API.
    /// * `model` — модель эмбеддингов (по умолчанию `text-embedding-3-small`).
    #[must_use]
    pub fn new(base_url: String, api_key: String, model: String) -> Self {
        let dimension = if model.contains("3-large") {
            3072
        } else {
            DEFAULT_EMBEDDING_DIM
        };

        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("valid reqwest client"),
            base_url,
            api_key,
            model,
            dimension,
        }
    }

    /// Создаёт с кастомной размерностью.
    #[must_use]
    pub fn with_dimension(
        base_url: String,
        api_key: String,
        model: String,
        dimension: usize,
    ) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("valid reqwest client"),
            base_url,
            api_key,
            model,
            dimension,
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        debug!(model = %self.model, text_len = text.len(), "requesting embedding");

        let payload = EmbeddingRequest {
            model: self.model.clone(),
            input: text,
        };

        let response: EmbeddingResponse = self
            .client
            .post(format!("{}/v1/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| SmithError::Memory {
                operation: "embed".to_string(),
                message: format!("HTTP request failed: {e}"),
            })?
            .json()
            .await
            .map_err(|e| SmithError::Memory {
                operation: "embed".to_string(),
                message: format!("JSON parse failed: {e}"),
            })?;

        let embedding = response
            .data
            .first()
            .ok_or_else(|| SmithError::Memory {
                operation: "embed".to_string(),
                message: "empty response data".to_string(),
            })?
            .embedding
            .clone();

        if embedding.len() != self.dimension {
            return Err(SmithError::Memory {
                operation: "embed".to_string(),
                message: format!(
                    "dimension mismatch: expected {}, got {}",
                    self.dimension,
                    embedding.len()
                ),
            });
        }

        debug!(dimension = embedding.len(), "embedding received");
        Ok(embedding)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

// ===================== DTOs =====================

#[derive(Debug, Serialize)]
struct EmbeddingRequest<'a> {
    model: String,
    input: &'a str,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}
