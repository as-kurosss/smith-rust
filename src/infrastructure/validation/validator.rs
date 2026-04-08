//! Валидация API-запросов.

use serde::{Deserialize, Serialize};

/// Ошибка валидации.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Поле, вызвавшее ошибку.
    pub field: String,
    /// Описание проблемы.
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Validation error on '{}': {}", self.field, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Валидирует тело API-запроса.
///
/// Проверяет:
/// - Максимальную длину строковых полей (4096 символов)
/// - Отсутствие null-байтов
/// - Максимальную глубину вложенности JSON (10 уровней)
///
/// # Errors
///
/// Возвращает [`ValidationError`] при нарушении ограничений.
pub fn validate_api_request(body: &serde_json::Value) -> Result<(), ValidationError> {
    validate_json_depth(body, 0, 10)?;
    validate_no_null_bytes(body)?;
    validate_string_lengths(body, 4096)?;
    Ok(())
}

fn validate_json_depth(
    value: &serde_json::Value,
    current: usize,
    max: usize,
) -> Result<(), ValidationError> {
    if current > max {
        return Err(ValidationError {
            field: "body".to_string(),
            message: format!("JSON nesting depth exceeds maximum ({max})"),
        });
    }

    match value {
        serde_json::Value::Object(map) => {
            for v in map.values() {
                validate_json_depth(v, current + 1, max)?;
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                validate_json_depth(v, current + 1, max)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_no_null_bytes(value: &serde_json::Value) -> Result<(), ValidationError> {
    if let Some(s) = value.as_str() {
        if s.contains('\0') {
            return Err(ValidationError {
                field: "body".to_string(),
                message: "null bytes are not allowed".to_string(),
            });
        }
    }
    if let Some(obj) = value.as_object() {
        for v in obj.values() {
            validate_no_null_bytes(v)?;
        }
    }
    if let Some(arr) = value.as_array() {
        for v in arr {
            validate_no_null_bytes(v)?;
        }
    }
    Ok(())
}

fn validate_string_lengths(
    value: &serde_json::Value,
    max_len: usize,
) -> Result<(), ValidationError> {
    if let Some(s) = value.as_str() {
        if s.len() > max_len {
            return Err(ValidationError {
                field: "body".to_string(),
                message: format!("string length {} exceeds maximum {max_len}", s.len()),
            });
        }
    }
    if let Some(obj) = value.as_object() {
        for (k, v) in obj {
            if k.len() > max_len {
                return Err(ValidationError {
                    field: format!("key '{k}'"),
                    message: format!("key length {} exceeds maximum {max_len}", k.len()),
                });
            }
            validate_string_lengths(v, max_len)?;
        }
    }
    if let Some(arr) = value.as_array() {
        for v in arr {
            validate_string_lengths(v, max_len)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_simple_object() {
        let body = json!({"message": "hello", "count": 42});
        assert!(validate_api_request(&body).is_ok());
    }

    #[test]
    fn test_validate_null_bytes() {
        let body = json!({"data": "hello\u{0000}world"});
        assert!(validate_api_request(&body).is_err());
    }

    #[test]
    fn test_validate_string_too_long() {
        let long_string: String = "a".repeat(5000);
        let body = json!({"content": long_string});
        let result = validate_api_request(&body);
        assert!(result.is_err());
        let err = result.expect_err("should fail");
        assert!(err.message.contains("4096"));
    }

    #[test]
    fn test_validate_deep_nesting() {
        let deep = json!({
            "a": {
                "b": {
                    "c": {
                        "d": {
                            "e": {
                                "f": {
                                    "g": {
                                        "h": {
                                            "i": {
                                                "j": {
                                                    "k": "too deep"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
        let result = validate_api_request(&deep);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_acceptable_depth() {
        let body = json!({
            "level1": {
                "level2": {
                    "level3": "ok"
                }
            }
        });
        assert!(validate_api_request(&body).is_ok());
    }
}
