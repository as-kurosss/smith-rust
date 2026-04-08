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
pub use application::retry_policy::with_retry;
pub use application::session_manager::SessionManager;
pub use application::tool_registry::ToolRegistry;
pub use domain::observability::{HealthStatus, RetryPolicy, SystemHealth};
pub use domain::session::{Session, SessionMetadata, SessionStore, SessionSummary};
pub use domain::{
    FunctionCall, LLMProvider, LLMResponse, Message, MessageRole, Role, Tool, ToolCall, ToolOutput,
};
pub use error::{Result, SmithError};
pub use infrastructure::health::HealthChecker;
pub use presentation::cli::{init_tracing, AppMode, CliArgs};

// Feature-gated реэкспорты
#[cfg(feature = "mock-llm")]
pub use infrastructure::llm::r#mock::MockLLMProvider;

#[cfg(feature = "openai")]
pub use infrastructure::llm::openai::OpenAIProvider;

#[cfg(feature = "memory")]
pub use application::context_manager::ContextManager;
#[cfg(feature = "memory")]
pub use domain::embedding::EmbeddingProvider;
#[cfg(feature = "memory")]
pub use domain::memory::{cosine_similarity, ChunkMetadata, MemoryChunk, MemoryStore};
#[cfg(feature = "memory")]
pub use infrastructure::embedding::openai::OpenAIEmbeddingProvider;
#[cfg(feature = "memory")]
pub use infrastructure::memory::json_store::JsonMemoryStore;
#[cfg(feature = "memory")]
pub use infrastructure::tools::memory_search::MemorySearchTool;

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
