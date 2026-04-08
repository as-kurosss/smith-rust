//! CLI-интерфейс на основе clap.
//!
//! Единственное место, где допускается вывод в stdout
//! для пользовательского взаимодействия (через std::io).

use std::path::PathBuf;

use clap::Parser;
use uuid::Uuid;

/// Режим работы приложения после парсинга аргументов.
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// Batch-режим: вывести список сессий и завершить работу.
    ListSessions,
    /// Batch-режим: загрузить сессию и завершить (или подготовить к save).
    LoadSession(Uuid),
    /// Batch-режим: сохранить активную сессию и завершить.
    SaveSession,
    /// Интерактивный режим: запустить чат-цикл.
    Interactive,
}

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

    // --- Session management flags ---
    /// Путь к директории хранения сессий.
    #[arg(long, env = "SMITH_STORAGE_PATH", default_value = "./sessions")]
    pub storage_path: PathBuf,

    /// Бэкенд хранения: `json`, `postgres`, `memory`.
    #[arg(long, default_value = "json", value_parser = ["json", "postgres", "memory"])]
    pub storage_backend: String,

    /// Загрузить сессию по UUID и завершить работу.
    #[arg(long)]
    pub session_load: Option<Uuid>,

    /// Сохранить активную сессию и завершить работу.
    #[arg(long)]
    pub session_save: bool,

    /// Вывести список всех сессий и завершить работу.
    #[arg(long)]
    pub session_list: bool,

    /// URL базы данных (только для postgres backend).
    #[arg(long, env = "DATABASE_URL", hide = true)]
    pub database_url: Option<String>,
}

impl CliArgs {
    /// Парсит аргументы из командной строки.
    #[must_use]
    pub fn parse_from_cli() -> Self {
        Self::parse()
    }

    /// Определяет режим работы приложения.
    #[must_use]
    pub fn mode(&self) -> AppMode {
        if self.session_list {
            AppMode::ListSessions
        } else if let Some(id) = self.session_load {
            AppMode::LoadSession(id)
        } else if self.session_save {
            AppMode::SaveSession
        } else {
            AppMode::Interactive
        }
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
        assert_eq!(args.storage_path, PathBuf::from("./sessions"));
        assert_eq!(args.storage_backend, "json");
        assert!(args.session_load.is_none());
        assert!(!args.session_save);
        assert!(!args.session_list);
        assert_eq!(args.mode(), AppMode::Interactive);
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

    #[test]
    fn test_session_load_mode() {
        let id = uuid::Uuid::new_v4();
        let args = CliArgs::parse_from(&["smith", "--session-load", &id.to_string()]);
        assert_eq!(args.mode(), AppMode::LoadSession(id));
    }

    #[test]
    fn test_session_list_mode() {
        let args = CliArgs::parse_from(&["smith", "--session-list"]);
        assert_eq!(args.mode(), AppMode::ListSessions);
    }

    #[test]
    fn test_session_save_mode() {
        let args = CliArgs::parse_from(&["smith", "--session-save"]);
        assert_eq!(args.mode(), AppMode::SaveSession);
    }
}
