//! Domain layer — чистые бизнес-сущности без внешних зависимостей.

pub mod llm;
pub mod message;
pub mod session;
pub mod tool;

pub use llm::LLMProvider;
pub use message::{FunctionCall, LLMResponse, Message, MessageRole, Role, ToolCall};
pub use session::{Session, SessionMetadata, SessionStore, SessionSummary};
pub use tool::{Tool, ToolOutput};
