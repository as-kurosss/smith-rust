//! Роли участников диалога, сообщения и типы для tool calls.
//!
//! Соответствует OpenAI Chat Completions API spec для обеспечения
//! совместимости с реальными LLM-провайдерами.

/// Роль отправителя сообщения.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// Сообщение от пользователя.
    User,
    /// Сообщение от LLM-агента.
    Assistant,
    /// Системное сообщение (инструкции, контекст).
    System,
    /// Результат выполнения инструмента.
    Tool,
}

/// Обратная совместимость: старый код может использовать `Role`.
pub type Role = MessageRole;

/// Вызов функции от LLM-агента.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionCall {
    /// Имя вызываемой функции (инструмента).
    pub name: String,
    /// Аргументы в виде JSON-строки. Десериализуется при выполнении.
    pub arguments: String,
}

/// Единичный tool call от LLM.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    /// Уникальный идентификатор вызова.
    pub id: String,
    /// Тип вызова (всегда "function").
    #[serde(rename = "type")]
    pub call_type: String,
    /// Информация о вызываемой функции.
    pub function: FunctionCall,
}

impl ToolCall {
    /// Создаёт новый tool call.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            call_type: "function".to_string(),
            function: FunctionCall {
                name: name.into(),
                arguments: arguments.into(),
            },
        }
    }
}

/// Отдельное сообщение в рамках диалога.
///
/// Поля `tool_calls`, `tool_call_id`, `name` соответствуют
/// OpenAI API и используются для обработки tool calls.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    /// Роль отправителя.
    pub role: MessageRole,
    /// Текстовое содержимое (None для tool call без текста).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Вызовы инструментов (только для role: assistant).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// ID вызова инструмента (только для role: tool).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Имя инструмента (только для role: tool).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Message {
    /// Создаёт сообщение от пользователя.
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    /// Создаёт сообщение от ассистента.
    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    /// Создаёт сообщение от ассистента с tool calls.
    #[must_use]
    pub fn assistant_with_tool_calls(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
            name: None,
        }
    }

    /// Создаёт системное сообщение.
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    /// Создаёт сообщение с результатом выполнения инструмента.
    #[must_use]
    pub fn tool_result(
        tool_call_id: impl Into<String>,
        name: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            role: MessageRole::Tool,
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
            name: Some(name.into()),
        }
    }

    /// Проверяет, является ли сообщение текстовым (имеет content).
    #[must_use]
    pub fn has_content(&self) -> bool {
        self.content.as_ref().is_some_and(|c| !c.is_empty())
    }

    /// Возвращает содержимое или пустую строку.
    #[must_use]
    pub fn content_or_empty(&self) -> &str {
        self.content.as_deref().unwrap_or("")
    }
}

/// Типизированный ответ от LLM-провайдера.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LLMResponse {
    /// Роль отвечающей стороны (всегда `MessageRole::Assistant`).
    pub role: MessageRole,
    /// Сгенерированное содержимое (может быть пустым при tool-only ответе).
    pub content: String,
    /// Вызовы инструментов (если LLM хочет выполнить инструменты).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl LLMResponse {
    /// Создаёт ответ от имени ассистента без tool calls.
    #[must_use]
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: None,
        }
    }

    /// Создаёт ответ с tool calls.
    #[must_use]
    pub fn with_tool_calls(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: Some(tool_calls),
        }
    }

    /// Проверяет, содержит ли ответ вызовы инструментов.
    #[must_use]
    pub fn has_tool_calls(&self) -> bool {
        self.tool_calls.as_ref().is_some_and(|tc| !tc.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_user_creation() {
        let msg = Message::user("hello");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, Some("hello".to_string()));
        assert!(msg.tool_calls.is_none());
    }

    #[test]
    fn test_message_serialization_roundtrip() {
        let msg = Message::user("test message");
        let json = serde_json::to_string(&msg).expect("serialize");
        let deserialized: Message = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.role, MessageRole::User);
        assert_eq!(deserialized.content, Some("test message".to_string()));
    }

    #[test]
    fn test_llm_response_serialization() {
        let resp = LLMResponse::new("assistant reply");
        let json = serde_json::to_string(&resp).expect("serialize");
        assert!(json.contains("assistant"));
        assert!(json.contains("assistant reply"));

        let deserialized: LLMResponse = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.role, MessageRole::Assistant);
        assert_eq!(deserialized.content, "assistant reply");
    }

    #[test]
    fn test_role_serialization() {
        assert_eq!(
            serde_json::to_string(&MessageRole::User).expect("serialize user"),
            "\"user\""
        );
        assert_eq!(
            serde_json::to_string(&MessageRole::Assistant).expect("serialize assistant"),
            "\"assistant\""
        );
        assert_eq!(
            serde_json::to_string(&MessageRole::System).expect("serialize system"),
            "\"system\""
        );
        assert_eq!(
            serde_json::to_string(&MessageRole::Tool).expect("serialize tool"),
            "\"tool\""
        );
    }

    #[test]
    fn test_tool_call_serialization() {
        let tc = ToolCall::new("call_123", "calculator", "{\"expression\": \"2+2\"}");
        let json = serde_json::to_string(&tc).expect("serialize");
        assert!(json.contains("call_123"));
        assert!(json.contains("calculator"));
        assert!(json.contains("function"));

        let deserialized: ToolCall = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.id, "call_123");
        assert_eq!(deserialized.function.name, "calculator");
    }

    #[test]
    fn test_tool_result_message() {
        let msg = Message::tool_result("call_123", "calculator", "4");
        assert_eq!(msg.role, MessageRole::Tool);
        assert_eq!(msg.tool_call_id, Some("call_123".to_string()));
        assert_eq!(msg.name, Some("calculator".to_string()));
        assert_eq!(msg.content, Some("4".to_string()));
    }

    #[test]
    fn test_llm_response_with_tool_calls() {
        let tool_calls = vec![ToolCall::new("call_1", "echo", "{\"text\": \"hi\"}")];
        let resp = LLMResponse::with_tool_calls("", tool_calls.clone());
        assert!(resp.has_tool_calls());
        assert_eq!(resp.tool_calls.as_ref().expect("tool_calls").len(), 1);
    }

    #[test]
    fn test_message_content_or_empty() {
        let msg = Message::user("hello");
        assert_eq!(msg.content_or_empty(), "hello");

        let msg_with_none = Message {
            role: MessageRole::Assistant,
            content: None,
            tool_calls: Some(vec![]),
            tool_call_id: None,
            name: None,
        };
        assert_eq!(msg_with_none.content_or_empty(), "");
    }
}
