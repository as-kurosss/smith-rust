//! smith-rust — production-ready AI agent framework.
//!
//! # Architecture
//!
//! Проект следует слоистой архитектуре:
//! - **domain** — чистые бизнес-сущности (Message, LLMProvider trait)
//! - **application** — use-cases (chat_loop)
//! - **infrastructure** — внешние интеграции (mock LLM)
//! - **presentation** — пользовательский интерфейс (CLI)

// Модули
pub mod application;
pub mod domain;
pub mod error;
pub mod infrastructure;
pub mod presentation;

// Публичные реэкспорты верхнего уровня
pub use application::chat_loop::{run_chat_loop, ChatConfig, ChatSession};
pub use domain::{LLMProvider, LLMResponse, Message, Role};
pub use error::{Result, SmithError};
pub use presentation::cli::{init_tracing, CliArgs};

// Feature-gated реэкспорты
#[cfg(feature = "mock-llm")]
pub use infrastructure::llm::r#mock::MockLLMProvider;

#[cfg(feature = "openai")]
pub use infrastructure::llm::openai::OpenAIProvider;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_api_available() {
        // Проверяем, что все публичные типы доступны
        fn _assert_types_exist<T>() {}
        _assert_types_exist::<Message>();
        _assert_types_exist::<LLMResponse>();
        _assert_types_exist::<Role>();
        _assert_types_exist::<ChatConfig>();
    }
}
