//! Трейт для инструментов и тип результата выполнения.
//!
//! Каждый инструмент реализует [`Tool`], определяя имя, описание,
//! JSON-схему параметров и логику выполнения.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Результат выполнения инструмента.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    /// Текстовый результат (сообщение для LLM).
    pub content: String,
    /// Флаг успешного выполнения.
    pub success: bool,
}

impl ToolOutput {
    /// Создаёт успешный результат.
    #[must_use]
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            success: true,
        }
    }

    /// Создаёт результат с ошибкой.
    #[must_use]
    pub fn error(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            success: false,
        }
    }
}

/// Трейт для инструментов, вызываемых LLM.
///
/// Все реализации должны быть `Send + Sync` для использования
/// в асинхронном контексте и параллельного выполнения.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Уникальное имя инструмента (используется LLM для вызова).
    fn name(&self) -> &str;

    /// Описание для LLM (помогает модели выбрать инструмент).
    fn description(&self) -> &str;

    /// JSON Schema параметров (описание ожидаемых аргументов).
    fn schema(&self) -> serde_json::Value;

    /// Выполняет инструмент с указанными параметрами.
    ///
    /// # Arguments
    ///
    /// * `params` — десериализованные JSON-параметры вызова.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку при невалидных параметрах или сбое выполнения.
    async fn execute(&self, params: serde_json::Value) -> Result<ToolOutput>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_output_success() {
        let output = ToolOutput::success("result data");
        assert!(output.success);
        assert_eq!(output.content, "result data");
    }

    #[test]
    fn test_tool_output_error() {
        let output = ToolOutput::error("something went wrong");
        assert!(!output.success);
        assert_eq!(output.content, "something went wrong");
    }
}
