//! Domain layer — чистые бизнес-сущности без внешних зависимостей.

pub mod chat_event;
pub mod embedding;
pub mod llm;
pub mod memory;
pub mod message;
pub mod observability;
#[cfg(feature = "security")]
pub mod security;
pub mod session;
pub mod tool;

pub use chat_event::ChatEvent;
pub use embedding::EmbeddingProvider;
pub use llm::LLMProvider;
pub use memory::{cosine_similarity, ChunkMetadata, MemoryChunk, MemoryStore};
pub use message::{FunctionCall, LLMResponse, Message, MessageRole, Role, ToolCall};
pub use observability::{HealthStatus, RetryPolicy, SystemHealth};
#[cfg(feature = "security")]
pub use security::{AuditEvent, SanitizationAction, Secret, SecretProvider, SecurityError};
pub use session::{Session, SessionMetadata, SessionStore, SessionSummary};
pub use tool::{Tool, ToolOutput};
