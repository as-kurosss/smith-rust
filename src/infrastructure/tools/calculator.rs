//! CalculatorTool — безопасное вычисление арифметических выражений.
//!
//! Поддерживает: `+ - * / ( )`, целые и вещественные числа.
//! Не поддерживает: переменные, функции, научную нотацию, строковые литералы.
//! Реализация: рекурсивный спуск (без eval).

use async_trait::async_trait;
use serde_json::json;

use crate::domain::tool::{Tool, ToolOutput};
use crate::error::{Result, SmithError};

/// Инструмент для безопасного вычисления арифметических выражений.
#[derive(Debug, Clone, Default)]
pub struct CalculatorTool;

impl CalculatorTool {
    /// Создаёт новый экземпляр.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Валидирует входную строку: разрешены только цифры, операторы, скобки, пробелы, точка.
    fn validate_input(expr: &str) -> Result<()> {
        let allowed = |c: char| c.is_ascii_digit() || " +-*/.()".contains(c);
        if !expr.chars().all(allowed) {
            return Err(SmithError::ToolExecution {
                tool_name: "calculator".to_string(),
                message: format!(
                    "invalid characters in expression. Only digits, + - * / ( ) . and spaces allowed. Got: {expr}"
                ),
            });
        }
        if expr.trim().is_empty() {
            return Err(SmithError::ToolExecution {
                tool_name: "calculator".to_string(),
                message: "empty expression".to_string(),
            });
        }
        Ok(())
    }

    /// Вычисляет выражение через рекурсивный спуск.
    fn evaluate(expr: &str) -> Result<f64> {
        let mut parser = ExprParser::new(expr);
        let result = parser.parse_expression()?;
        if !parser.is_at_end() {
            return Err(SmithError::ToolExecution {
                tool_name: "calculator".to_string(),
                message: format!("unexpected trailing content: {}", &expr[parser.pos..]),
            });
        }
        Ok(result)
    }
}

/// Парсер выражений методом рекурсивного спуска.
///
/// Грамматика:
/// ```text
/// expression → term (('+' | '-') term)*
/// term       → factor (('*' | '/') factor)*
/// factor     → NUMBER | '(' expression ')' | ('-' | '+') factor
/// ```
struct ExprParser {
    tokens: Vec<Token>,
    pos: usize,
}

#[derive(Debug, Clone)]
enum Token {
    Number(f64),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
}

impl ExprParser {
    fn new(input: &str) -> Self {
        let tokens = tokenize(input);
        Self { tokens, pos: 0 }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let token = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(token)
        } else {
            None
        }
    }

    fn parse_expression(&mut self) -> Result<f64> {
        let mut left = self.parse_term()?;
        while let Some(token) = self.peek() {
            match token {
                Token::Plus => {
                    self.advance();
                    let right = self.parse_term()?;
                    left += right;
                }
                Token::Minus => {
                    self.advance();
                    let right = self.parse_term()?;
                    left -= right;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<f64> {
        let mut left = self.parse_factor()?;
        while let Some(token) = self.peek() {
            match token {
                Token::Star => {
                    self.advance();
                    let right = self.parse_factor()?;
                    left *= right;
                }
                Token::Slash => {
                    self.advance();
                    let right = self.parse_factor()?;
                    if right.abs() < f64::EPSILON {
                        return Err(SmithError::ToolExecution {
                            tool_name: "calculator".to_string(),
                            message: "division by zero".to_string(),
                        });
                    }
                    left /= right;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<f64> {
        match self.advance() {
            Some(Token::Number(n)) => Ok(n),
            Some(Token::LParen) => {
                let result = self.parse_expression()?;
                match self.advance() {
                    Some(Token::RParen) => Ok(result),
                    _ => Err(SmithError::ToolExecution {
                        tool_name: "calculator".to_string(),
                        message: "missing closing parenthesis".to_string(),
                    }),
                }
            }
            Some(Token::Minus) => {
                let result = self.parse_factor()?;
                Ok(-result)
            }
            Some(Token::Plus) => self.parse_factor(),
            Some(other) => Err(SmithError::ToolExecution {
                tool_name: "calculator".to_string(),
                message: format!("unexpected token: {other:?}"),
            }),
            None => Err(SmithError::ToolExecution {
                tool_name: "calculator".to_string(),
                message: "unexpected end of expression".to_string(),
            }),
        }
    }
}

fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' => {}
            '+' => tokens.push(Token::Plus),
            '-' => tokens.push(Token::Minus),
            '*' => tokens.push(Token::Star),
            '/' => tokens.push(Token::Slash),
            '(' => tokens.push(Token::LParen),
            ')' => tokens.push(Token::RParen),
            c if c.is_ascii_digit() || c == '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let num_str: String = chars[start..i].iter().collect();
                if let Ok(n) = num_str.parse::<f64>() {
                    tokens.push(Token::Number(n));
                }
                i -= 1; // компенсация инкремента в конце цикла
            }
            _ => {}
        }
        i += 1;
    }
    tokens
}

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }

    fn description(&self) -> &str {
        "Evaluates a mathematical expression. Supports: + - * / ( ), integers and floats. No variables or functions."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "Arithmetic expression to evaluate. E.g. '2 + 3 * (4 - 1)'"
                }
            },
            "required": ["expression"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolOutput> {
        let expression = params
            .get("expression")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SmithError::ToolExecution {
                tool_name: self.name().to_string(),
                message: "missing or invalid 'expression' parameter (must be a string)".to_string(),
            })?;

        CalculatorTool::validate_input(expression)?;
        let result = CalculatorTool::evaluate(expression)?;

        // Форматируем: целые числа без десятичной точки
        let output = if result.fract().abs() < f64::EPSILON && result.abs() < 1e15 {
            format!("{}", result as i64)
        } else {
            format!("{result}")
        };

        Ok(ToolOutput::success(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_addition() {
        let tool = CalculatorTool::new();
        let output = tool
            .execute(json!({"expression": "2 + 3"}))
            .await
            .expect("execute");
        assert_eq!(output.content, "5");
    }

    #[tokio::test]
    async fn test_precedence() {
        let tool = CalculatorTool::new();
        let output = tool
            .execute(json!({"expression": "2 + 3 * 4"}))
            .await
            .expect("execute");
        assert_eq!(output.content, "14");
    }

    #[tokio::test]
    async fn test_parentheses() {
        let tool = CalculatorTool::new();
        let output = tool
            .execute(json!({"expression": "(2 + 3) * 4"}))
            .await
            .expect("execute");
        assert_eq!(output.content, "20");
    }

    #[tokio::test]
    async fn test_division_by_zero() {
        let tool = CalculatorTool::new();
        let result = tool.execute(json!({"expression": "1 / 0"})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_characters() {
        let tool = CalculatorTool::new();
        let result = tool.execute(json!({"expression": "2 + sin(3)"})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_missing_expression() {
        let tool = CalculatorTool::new();
        let result = tool.execute(json!({"other": "param"})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_float_result() {
        let tool = CalculatorTool::new();
        let output = tool
            .execute(json!({"expression": "7 / 2"}))
            .await
            .expect("execute");
        assert_eq!(output.content, "3.5");
    }

    #[tokio::test]
    async fn test_negative_number() {
        let tool = CalculatorTool::new();
        let output = tool
            .execute(json!({"expression": "-5 + 3"}))
            .await
            .expect("execute");
        assert_eq!(output.content, "-2");
    }
}
