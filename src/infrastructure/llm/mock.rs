//! Mock-реализация LLM-провайдера для тестирования.
//!
//! Возвращает детерминированный ответ вида:
//! `[MOCK] Response to: "{user_input}"`

use async_trait::async_trait;
use tracing::debug;

use crate::domain::message::{LLMResponse, Message};
use crate::domain::LLMProvider as LLMProviderTrait;
use crate::error::Result;

/// Mock-провайдер, не выполняющий реальных HTTP-вызовов.
#[derive(Debug, Clone)]
pub struct MockLLMProvider;

impl MockLLMProvider {
    /// Создаёт новый экземпляр.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockLLMProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LLMProviderTrait for MockLLMProvider {
    async fn chat(&self, messages: &[Message]) -> Result<LLMResponse> {
        // Находим последнее сообщение пользователя
        let user_input = messages
            .iter()
            .rev()
            .find(|m| m.role == crate::domain::message::MessageRole::User)
            .map(|m| m.content_or_empty())
            .filter(|s| !s.is_empty())
            .unwrap_or("<empty>");

        // Санитизируем ввод перед логированием
        let sanitized =
            crate::infrastructure::validation::sanitizer::sanitize_for_logging(user_input);
        let content = format!("[MOCK] Response to: \"{sanitized}\"");
        debug!(%content, "Mock LLM generated response");

        // Имитация сетевой задержки (1мс)
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

        Ok(LLMResponse::new(content))
    }
}

// Проверяем, что MockLLMProvider реализует Send + Sync
fn _assert_send_sync<T: Send + Sync>() {}
fn _mock_is_send_sync() {
    _assert_send_sync::<MockLLMProvider>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_response() {
        let provider = MockLLMProvider::new();
        let messages = vec![Message::user("hello world")];
        let response = provider.chat(&messages).await.expect("chat should succeed");

        assert_eq!(
            response.role,
            crate::domain::message::MessageRole::Assistant
        );
        assert_eq!(response.content, "[MOCK] Response to: \"hello world\"");
    }

    #[tokio::test]
    async fn test_mock_empty_input() {
        let provider = MockLLMProvider::new();
        let messages: Vec<Message> = vec![];
        let response = provider.chat(&messages).await.expect("chat should succeed");

        assert_eq!(response.content, "[MOCK] Response to: \"<empty>\"");
    }

    #[tokio::test]
    async fn test_mock_with_system_message() {
        let provider = MockLLMProvider::new();
        let messages = vec![
            Message::system("You are a helpful assistant."),
            Message::user("What is 2+2?"),
        ];
        let response = provider.chat(&messages).await.expect("chat should succeed");

        assert_eq!(response.content, "[MOCK] Response to: \"What is 2+2?\"");
    }
}
