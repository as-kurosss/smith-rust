//! Presentation layer — пользовательский интерфейс (CLI, TUI).

pub mod cli;

#[cfg(feature = "tui")]
pub mod tui;
