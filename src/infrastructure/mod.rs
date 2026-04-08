//! Infrastructure layer — внешние интеграции (LLM-провайдеры, хранилища, HTTP).

pub mod health;
pub mod llm;

#[cfg(feature = "memory")]
pub mod embedding;
#[cfg(feature = "memory")]
pub mod memory;

#[cfg(feature = "observability")]
pub mod metrics;

#[cfg(feature = "security")]
pub mod secrets;
pub mod storage;
pub mod tools;
#[cfg(feature = "security")]
pub mod validation;
