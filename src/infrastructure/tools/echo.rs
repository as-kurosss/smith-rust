//! EchoTool — возвращает входные параметры как сериализованную строку.
//!
//! Полезен для отладки и тестирования tool dispatch.

use async_trait::async_trait;
use serde_json::json;

use crate::domain::tool::{Tool, ToolOutput};
use crate::error::Result;

/// Инструмент, возвращающий входные параметры как JSON-строку.
#[derive(Debug, Clone, Default)]
pub struct EchoTool;

impl EchoTool {
    /// Создаёт новый экземпляр.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Returns the input arguments as a JSON string. Useful for debugging."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {},
            "description": "Accepts any JSON arguments and returns them as a string."
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolOutput> {
        let result = serde_json::to_string_pretty(&params).map_err(|e| {
            crate::error::SmithError::ToolExecution {
                tool_name: self.name().to_string(),
                message: format!("serialization failed: {e}"),
            }
        })?;
        Ok(ToolOutput::success(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_echo_with_object() {
        let tool = EchoTool::new();
        let params = serde_json::json!({"key": "value", "num": 42});
        let output = tool.execute(params).await.expect("execute should succeed");
        assert!(output.success);
        assert!(output.content.contains("key"));
        assert!(output.content.contains("value"));
        assert!(output.content.contains("42"));
    }

    #[tokio::test]
    async fn test_echo_with_null() {
        let tool = EchoTool::new();
        let output = tool
            .execute(serde_json::Value::Null)
            .await
            .expect("execute");
        assert!(output.success);
        assert_eq!(output.content, "null");
    }
}
