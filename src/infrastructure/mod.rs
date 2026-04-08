//! Infrastructure layer — внешние интеграции (LLM-провайдеры, хранилища, HTTP).

pub mod health;
pub mod llm;

#[cfg(feature = "memory")]
pub mod embedding;
#[cfg(feature = "memory")]
pub mod memory;

#[cfg(feature = "observability")]
pub mod metrics;

pub mod storage;
pub mod tools;
