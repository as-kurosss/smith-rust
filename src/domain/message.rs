/// Роли участников диалога.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Сообщение от пользователя.
    User,
    /// Сообщение от LLM-агента.
    Assistant,
    /// Системное сообщение (инструкции, контекст).
    System,
}

/// Отдельное сообщение в рамках диалога.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    /// Роль отправителя.
    pub role: Role,
    /// Текстовое содержимое.
    pub content: String,
}

impl Message {
    /// Создаёт сообщение от пользователя.
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    /// Создаёт сообщение от ассистента.
    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }

    /// Создаёт системное сообщение.
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }
}

/// Типизированный ответ от LLM-провайдера.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LLMResponse {
    /// Роль отвечающей стороны (всегда `Role::Assistant`).
    pub role: Role,
    /// Сгенерированное содержимое.
    pub content: String,
}

impl LLMResponse {
    /// Создаёт ответ от имени ассистента.
    #[must_use]
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_user_creation() {
        let msg = Message::user("hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "hello");
    }

    #[test]
    fn test_message_serialization_roundtrip() {
        let msg = Message::user("test message");
        let json = serde_json::to_string(&msg).expect("serialize");
        let deserialized: Message = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.role, Role::User);
        assert_eq!(deserialized.content, "test message");
    }

    #[test]
    fn test_llm_response_serialization() {
        let resp = LLMResponse::new("assistant reply");
        let json = serde_json::to_string(&resp).expect("serialize");
        assert!(json.contains("assistant"));
        assert!(json.contains("assistant reply"));

        let deserialized: LLMResponse = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.role, Role::Assistant);
        assert_eq!(deserialized.content, "assistant reply");
    }

    #[test]
    fn test_role_serialization() {
        assert_eq!(
            serde_json::to_string(&Role::User).expect("serialize user"),
            "\"user\""
        );
        assert_eq!(
            serde_json::to_string(&Role::Assistant).expect("serialize assistant"),
            "\"assistant\""
        );
        assert_eq!(
            serde_json::to_string(&Role::System).expect("serialize system"),
            "\"system\""
        );
    }
}
