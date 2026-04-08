//! Application layer — use-cases и оркестрация (чат-цикл, выполнение инструментов).

pub mod chat_loop;
pub mod session_manager;
pub mod tool_registry;

pub use session_manager::SessionManager;
pub use tool_registry::ToolRegistry;
