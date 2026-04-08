//! CLI-интерфейс на основе clap.
//!
//! Единственное место, где допускается вывод в stdout
//! для пользовательского взаимодействия (через std::io).

use clap::Parser;

/// smith — CLI-приложение для общения с LLM-агентом.
#[derive(Parser, Debug)]
#[command(name = "smith")]
#[command(version = "0.1.0")]
#[command(about = "Interactive LLM chat agent", long_about = None)]
pub struct CliArgs {
    /// Системный промпт (инструкции для модели).
    #[arg(short, long, default_value = "You are a helpful assistant.")]
    pub system_prompt: String,

    /// Максимальное количество сообщений в истории.
    #[arg(long, default_value_t = 50)]
    pub max_history: usize,

    /// Имя модели (например, gpt-3.5-turbo, gpt-4).
    #[arg(long, default_value = "gpt-3.5-turbo")]
    pub model: String,

    /// Base URL для OpenAI-совместимого API.
    #[arg(long, default_value = "https://api.openai.com")]
    pub base_url: String,

    /// Включить mock-режим LLM (без реальных API-вызовов).
    #[arg(long, default_value_t = true)]
    pub mock: bool,

    /// Уровень логирования (RUST_LOG override).
    #[arg(long, default_value = "smith_rust=info")]
    pub log_level: String,
}

impl CliArgs {
    /// Парсит аргументы из командной строки.
    #[must_use]
    pub fn parse_from_cli() -> Self {
        Self::parse()
    }
}

/// Инициализирует tracing-subscriber с форматированным выводом в терминал.
///
/// # Panics
///
/// Паникует при невозможности инициализации (только на старте приложения).
pub fn init_tracing(log_level: &str) {
    use tracing_subscriber::EnvFilter;

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::try_new(log_level).expect("valid log level"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .compact()
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_args_defaults() {
        let args = CliArgs::parse_from(&["smith"]);
        assert_eq!(args.system_prompt, "You are a helpful assistant.");
        assert_eq!(args.max_history, 50);
        assert_eq!(args.model, "gpt-3.5-turbo");
        assert_eq!(args.base_url, "https://api.openai.com");
        assert!(args.mock);
        assert_eq!(args.log_level, "smith_rust=info");
    }

    #[test]
    fn test_cli_args_custom() {
        let args = CliArgs::parse_from(&[
            "smith",
            "--system-prompt",
            "Custom prompt",
            "--max-history",
            "10",
            "--model",
            "gpt-4",
            "--base-url",
            "http://localhost:8080",
            "--log-level",
            "debug",
        ]);
        assert_eq!(args.system_prompt, "Custom prompt");
        assert_eq!(args.max_history, 10);
        assert_eq!(args.model, "gpt-4");
        assert_eq!(args.base_url, "http://localhost:8080");
        assert_eq!(args.log_level, "debug");
    }
}
