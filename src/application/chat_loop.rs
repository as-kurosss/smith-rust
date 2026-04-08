//! Асинхронный цикл чата — оркестрация ввода, LLM и вывода.
//!
//! Этот модуль отвечает за:
//! 1. Чтение ввода пользователя.
//! 2. Передачу истории сообщений LLM-провайдеру.
//! 3. Логирование ответа.
//!
//! Сам по себе не выполняет I/O с терминалом —
//! Reader/Writer передаются как обобщённые параметры.

use std::io::{BufRead, Write};

use tracing::{debug, error, info};

use crate::domain::message::{LLMResponse, Message};
use crate::domain::LLMProvider;
use crate::error::{Result, SmithError};

/// Конфигурация чат-цикла.
#[derive(Debug, Clone)]
pub struct ChatConfig {
    /// Максимальное количество сообщений в истории (защита от переполнения).
    pub max_history: usize,
    /// Системное сообщение (инструкции для модели).
    pub system_prompt: Option<String>,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            max_history: 50,
            system_prompt: Some("You are a helpful assistant.".to_string()),
        }
    }
}

/// Чат-сессия с хранением истории.
pub struct ChatSession<P: LLMProvider> {
    provider: P,
    history: Vec<Message>,
    config: ChatConfig,
}

impl<P: LLMProvider> ChatSession<P> {
    /// Создаёт новую сессию с указанным провайдером.
    #[must_use]
    pub fn new(provider: P, config: ChatConfig) -> Self {
        let mut history = Vec::new();

        if let Some(prompt) = &config.system_prompt {
            history.push(Message::system(prompt.clone()));
        }

        Self {
            provider,
            history,
            config,
        }
    }

    /// Обрабатывает одно сообщение пользователя и возвращает ответ LLM.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку при сбое LLM-провайдера или переполнении истории.
    pub async fn process_message(&mut self, user_input: &str) -> Result<LLMResponse> {
        if user_input.trim().is_empty() {
            return Err(SmithError::InvalidInput("empty input ignored".to_string()));
        }

        // Ограничение истории: перед добавлением нового сообщения
        // проверяем, что после добавления user+assistant (2 сообщения)
        // не превысим max_history. Если превысим — удаляем oldest.
        // При наличии system prompt он всегда остаётся на позиции 0.
        let capacity_for_new = if self.config.system_prompt.is_some() {
            1
        } else {
            0
        };
        let max_messages = self.config.max_history;

        while self.history.len() + 2 > max_messages {
            // Удаляем oldest non-system message
            if capacity_for_new > 0 && self.history.len() > 1 {
                self.history.remove(1);
            } else if self.history.len() > 1 {
                self.history.remove(0);
            } else {
                break;
            }
        }

        let user_msg = Message::user(user_input);
        self.history.push(user_msg);
        debug!(history_len = self.history.len(), "message added to history");

        // Вызов LLM с таймаутом
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            self.provider.chat(&self.history),
        )
        .await
        .map_err(|_| SmithError::LLM("LLM request timed out".to_string()))??;

        info!(role = ?response.role, content = %response.content, "LLM response received");

        self.history.push(Message::assistant(&response.content));

        Ok(response)
    }

    /// Возвращает текущую историю сообщений.
    #[must_use]
    pub fn history(&self) -> &[Message] {
        &self.history
    }

    /// Возвращает ссылку на провайдер.
    #[must_use]
    pub fn provider(&self) -> &P {
        &self.provider
    }
}

/// Запускает интерактивный цикл чата.
///
/// Читает строки из `reader`, отправляет в LLM, результат пишет в `writer`.
/// Цикл завершается при получении строки "exit" / "quit" или EOF.
///
/// # Errors
///
/// Возвращает ошибку при сбое I/O или LLM.
pub async fn run_chat_loop<P: LLMProvider, R: BufRead, W: Write>(
    session: &mut ChatSession<P>,
    reader: R,
    mut writer: W,
) -> Result<()> {
    info!("chat loop started");

    for line in reader.lines() {
        let input = line?;
        let trimmed = input.trim();

        if trimmed.eq_ignore_ascii_case("exit") || trimmed.eq_ignore_ascii_case("quit") {
            info!("chat loop terminated by user command");
            break;
        }

        if trimmed.is_empty() {
            debug!("empty input skipped");
            continue;
        }

        debug!(input = %trimmed, "user input received");
        writeln!(writer, "Thinking...").map_err(SmithError::from)?;

        match session.process_message(trimmed).await {
            Ok(response) => {
                writeln!(writer, "Assistant: {}", response.content).map_err(SmithError::from)?;
            }
            Err(ref e) => {
                error!(error = %e, "LLM processing failed");
                writeln!(writer, "Error: {e}").map_err(SmithError::from)?;
            }
        }

        writer.flush().map_err(SmithError::from)?;

        // Yield для предотвращения starvation в tokio
        tokio::task::yield_now().await;
    }

    info!("chat loop finished");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::llm::r#mock::MockLLMProvider;

    #[tokio::test]
    async fn test_process_message() {
        let provider = MockLLMProvider::new();
        let config = ChatConfig::default();
        let mut session = ChatSession::new(provider, config);

        let response = session
            .process_message("hello")
            .await
            .expect("should succeed");
        assert!(response.content.contains("[MOCK]"));
        assert!(response.content.contains("hello"));
        assert_eq!(session.history().len(), 3); // system + user + assistant
    }

    #[tokio::test]
    async fn test_empty_input_rejected() {
        let provider = MockLLMProvider::new();
        let config = ChatConfig::default();
        let mut session = ChatSession::new(provider, config);

        let result = session.process_message("").await;
        assert!(result.is_err());
        assert_eq!(session.history().len(), 1); // только system
    }

    #[tokio::test]
    async fn test_history_limit() {
        let provider = MockLLMProvider::new();
        let config = ChatConfig {
            max_history: 5,
            system_prompt: Some("system".to_string()),
        };
        let mut session = ChatSession::new(provider, config);

        // system + 4 pairs (user+assistant) = 9, но max=5, значит обрежет
        for i in 0..4 {
            session
                .process_message(&format!("msg{i}"))
                .await
                .expect("should succeed");
        }

        assert!(session.history().len() <= 5);
        // Первый элемент должен быть system
        assert_eq!(
            session.history()[0].role,
            crate::domain::message::Role::System
        );
    }

    #[tokio::test]
    async fn test_full_chat_session() {
        let provider = MockLLMProvider::new();
        let config = ChatConfig::default();
        let mut session = ChatSession::new(provider, config);

        let input = "What is the meaning of life?";
        let response = session
            .process_message(input)
            .await
            .expect("should succeed");

        assert!(response.content.contains(input));
        assert_eq!(session.history().len(), 3);
    }
}
