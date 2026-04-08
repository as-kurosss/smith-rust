//! Domain layer — чистые бизнес-сущности без внешних зависимостей.

pub mod embedding;
pub mod llm;
pub mod memory;
pub mod message;
pub mod session;
pub mod tool;

pub use embedding::EmbeddingProvider;
pub use llm::LLMProvider;
pub use memory::{cosine_similarity, ChunkMetadata, MemoryChunk, MemoryStore};
pub use message::{FunctionCall, LLMResponse, Message, MessageRole, Role, ToolCall};
pub use session::{Session, SessionMetadata, SessionStore, SessionSummary};
pub use tool::{Tool, ToolOutput};
