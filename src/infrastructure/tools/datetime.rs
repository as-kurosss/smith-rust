//! DateTimeTool — возвращает текущую дату/время в заданном формате.
//!
//! Поддерживает опциональный параметр `format` (strftime-совместимый).
//! По умолчанию: `%Y-%m-%d %H:%M:%S`.

use async_trait::async_trait;
use chrono::Local;
use serde_json::json;

use crate::domain::tool::{Tool, ToolOutput};
use crate::error::{Result, SmithError};

/// Инструмент для получения текущего времени.
#[derive(Debug, Clone, Default)]
pub struct DateTimeTool;

impl DateTimeTool {
    /// Создаёт новый экземпляр.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for DateTimeTool {
    fn name(&self) -> &str {
        "datetime"
    }

    fn description(&self) -> &str {
        "Returns the current date and time. Accepts optional 'format' parameter (strftime format). Default: '%Y-%m-%d %H:%M:%S'."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "format": {
                    "type": "string",
                    "description": "strftime-compatible format string. Default: '%Y-%m-%d %H:%M:%S'."
                }
            }
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolOutput> {
        let format = params
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("%Y-%m-%d %H:%M:%S");

        let now = Local::now();
        let formatted = now.format(format).to_string();

        // Валидация: если формат некорректен, chrono вернёт строку с плейсхолдерами
        if formatted.contains('%') && formatted.contains("invalid") {
            return Err(SmithError::ToolExecution {
                tool_name: self.name().to_string(),
                message: format!("invalid format string: {format}"),
            });
        }

        Ok(ToolOutput::success(formatted))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_datetime_default_format() {
        let tool = DateTimeTool::new();
        let params = serde_json::json!({});
        let output = tool.execute(params).await.expect("execute should succeed");
        assert!(output.success);
        // Default format: YYYY-MM-DD HH:MM:SS
        assert!(output.content.len() >= 19);
    }

    #[tokio::test]
    async fn test_datetime_custom_format() {
        let tool = DateTimeTool::new();
        let params = serde_json::json!({"format": "%d/%m/%Y"});
        let output = tool.execute(params).await.expect("execute");
        assert!(output.success);
        assert!(output.content.contains('/'));
    }

    #[tokio::test]
    async fn test_datetime_null_params() {
        let tool = DateTimeTool::new();
        let output = tool
            .execute(serde_json::Value::Null)
            .await
            .expect("execute");
        assert!(output.success);
        assert!(output.content.len() >= 19);
    }
}
