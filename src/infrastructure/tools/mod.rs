//! Infrastructure-реализации инструментов для LLM.

pub mod calculator;
pub mod datetime;
pub mod echo;

#[cfg(feature = "memory")]
pub mod memory_search;
