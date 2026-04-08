//! Application layer — use-cases и оркестрация (чат-цикл, выполнение инструментов).

pub mod chat_loop;
#[cfg(feature = "memory")]
pub mod context_manager;
pub mod retry_policy;
pub mod session_manager;
pub mod tool_registry;

pub use retry_policy::with_retry;
pub use session_manager::SessionManager;
pub use tool_registry::ToolRegistry;

#[cfg(feature = "memory")]
pub use context_manager::ContextManager;
