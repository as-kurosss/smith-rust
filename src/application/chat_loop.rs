//! Асинхронный цикл чата — оркестрация ввода, LLM и выполнения инструментов.
//!
//! Этот модуль отвечает за:
//! 1. Чтение ввода пользователя.
//! 2. Передачу истории сообщений LLM-провайдеру.
//! 3. Обработку tool calls — dispatch → execution → injection.
//! 4. Логирование ответа.
//!
//! Сам по себе не выполняет I/O с терминалом —
//! Reader/Writer передаются как обобщённые параметры.

use std::io::{BufRead, Write};
use std::sync::Arc;

use tracing::{debug, error, info, warn};

use crate::application::tool_registry::ToolRegistry;
use crate::domain::message::{LLMResponse, Message, ToolCall};
use crate::domain::LLMProvider;
use crate::error::{Result, SmithError};

/// Конфигурация чат-цикла.
#[derive(Debug, Clone)]
pub struct ChatConfig {
    /// Максимальное количество сообщений в истории (защита от переполнения).
    pub max_history: usize,
    /// Системное сообщение (инструкции для модели).
    pub system_prompt: Option<String>,
    /// Реестр инструментов (None = инструменты отключены).
    pub tool_registry: Option<Arc<ToolRegistry>>,
    /// Максимум итераций tool calls за один запрос (защита от бесконечного цикла).
    pub max_tool_iterations: usize,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            max_history: 50,
            system_prompt: Some("You are a helpful assistant.".to_string()),
            tool_registry: None,
            max_tool_iterations: 5,
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

    /// Обрабатывает одно сообщение пользователя и возвращает финальный ответ LLM.
    ///
    /// Если LLM возвращает tool calls, выполняет их и повторяет запрос
    /// до получения финального ответа (без tool calls).
    ///
    /// # Errors
    ///
    /// Возвращает ошибку при сбое LLM-провайдера, переполнении истории
    /// или превышении лимита tool iterations.
    pub async fn process_message(&mut self, user_input: &str) -> Result<LLMResponse> {
        if user_input.trim().is_empty() {
            return Err(SmithError::InvalidInput("empty input ignored".to_string()));
        }

        self.trim_history_if_needed();

        let user_msg = Message::user(user_input);
        self.history.push(user_msg);
        debug!(history_len = self.history.len(), "message added to history");

        // Основной цикл: LLM → tool calls → execution → repeat
        let mut iteration = 0;
        let max_iterations = self.config.max_tool_iterations;

        loop {
            iteration += 1;
            if iteration > max_iterations {
                return Err(SmithError::ToolLoopDetected { max_iterations });
            }

            // Вызов LLM с таймаутом
            let response = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                self.provider.chat(&self.history),
            )
            .await
            .map_err(|_| SmithError::LLM("LLM request timed out".to_string()))??;

            // Проверяем наличие tool calls
            let tool_calls = response.tool_calls.clone();
            let has_tool_calls = tool_calls.as_ref().is_some_and(|tc| !tc.is_empty());

            if has_tool_calls {
                let tool_calls = tool_calls.expect("tool_calls is some");
                info!(
                    iteration,
                    tool_call_count = tool_calls.len(),
                    "LLM returned tool calls"
                );
                let content = if response.content.is_empty() {
                    None
                } else {
                    Some(response.content.clone())
                };
                self.execute_and_inject_tool_results(&tool_calls, content)
                    .await?;
                continue; // Повторяем запрос с обновлённой историей
            }

            // Финальный ответ без tool calls
            info!(role = ?response.role, content = %response.content, "LLM response received");
            self.history.push(Message::assistant(&response.content));
            return Ok(response);
        }
    }

    /// Выполняет tool calls и добавляет результаты в историю.
    async fn execute_and_inject_tool_results(
        &mut self,
        tool_calls: &[ToolCall],
        assistant_content: Option<String>,
    ) -> Result<()> {
        // Добавляем assistant message с tool_calls в историю
        let assistant_msg =
            Message::assistant_with_tool_calls(assistant_content, tool_calls.to_vec());
        self.history.push(assistant_msg);

        // Выполняем каждый tool call
        let registry = self.config.tool_registry.as_ref().ok_or_else(|| {
            SmithError::InvalidState("tool calls returned but no registry configured".to_string())
        })?;

        for tc in tool_calls {
            debug!(
                tool_id = %tc.id,
                tool_name = %tc.function.name,
                arguments = %tc.function.arguments,
                "executing tool call"
            );

            let params = serde_json::from_str(&tc.function.arguments).unwrap_or_else(|e| {
                warn!(error = %e, "failed to parse tool arguments, using empty object");
                serde_json::json!({})
            });

            let tool_name = tc.function.name.clone();
            let tool_call_id = tc.id.clone();

            let output = match registry.execute(&tool_name, params).await {
                Ok(out) => out,
                Err(e) => {
                    error!(tool_name, error = %e, "tool execution failed");
                    crate::domain::tool::ToolOutput::error(format!("{e}"))
                }
            };

            debug!(
                tool_name,
                success = output.success,
                content_len = output.content.len(),
                "tool execution completed"
            );

            // Инжектим результат как tool message
            let result_msg = Message::tool_result(&tool_call_id, &tool_name, &output.content);
            self.history.push(result_msg);
        }

        Ok(())
    }

    /// Обрезает историю если превышает лимит.
    fn trim_history_if_needed(&mut self) {
        let capacity_for_new = if self.config.system_prompt.is_some() {
            1
        } else {
            0
        };
        let max_messages = self.config.max_history;

        while self.history.len() + 2 > max_messages {
            if capacity_for_new > 0 && self.history.len() > 1 {
                self.history.remove(1);
            } else if self.history.len() > 1 {
                self.history.remove(0);
            } else {
                break;
            }
        }
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
            ..ChatConfig::default()
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
            crate::domain::message::MessageRole::System
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
