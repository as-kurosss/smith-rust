//! Infrastructure-реализации LLM-провайдеров.

#[cfg(feature = "mock-llm")]
pub mod r#mock;

#[cfg(feature = "openai")]
pub mod openai;
