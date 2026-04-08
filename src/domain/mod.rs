//! Domain layer — чистые бизнес-сущности без внешних зависимостей.

pub mod llm;
pub mod message;

pub use llm::LLMProvider;
pub use message::{LLMResponse, Message, Role};
