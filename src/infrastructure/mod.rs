//! Infrastructure layer — внешние интеграции (LLM-провайдеры, хранилища, HTTP).

pub mod llm;

#[cfg(feature = "memory")]
pub mod embedding;
#[cfg(feature = "memory")]
pub mod memory;

pub mod storage;
pub mod tools;
