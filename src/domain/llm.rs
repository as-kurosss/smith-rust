//! Трейт для LLM-провайдеров (инверсия зависимостей).
//!
//! Высокоуровневая логика зависит от этой абстракции,
//! а не от конкретных реализаций (OpenAI, mock и т.д.).

use async_trait::async_trait;

use crate::domain::message::{LLMResponse, Message};
use crate::error::Result;

/// Абстрактный LLM-провайдер.
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Отправляет историю сообщений и получает ответ от модели.
    ///
    /// # Arguments
    ///
    /// * `messages` — история диалога (system + user + assistant).
    ///
    /// # Errors
    ///
    /// Возвращает [`SmithError`](crate::error::SmithError) при сетевых сбоях,
    /// невалидном ответе или внутренних ошибках провайдера.
    async fn chat(&self, messages: &[Message]) -> Result<LLMResponse>;
}

/// Реализация для `Box<dyn LLMProvider>` — позволяет использовать trait objects.
#[async_trait]
impl LLMProvider for Box<dyn LLMProvider> {
    async fn chat(&self, messages: &[Message]) -> Result<LLMResponse> {
        self.as_ref().chat(messages).await
    }
}
