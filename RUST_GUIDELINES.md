# Rust Language Guidelines for smith-rust

## 🎯 Naming Conventions
```rust
// Types: PascalCase
pub struct ChatSession;
pub trait LLMProvider;

// Functions/Methods: snake_case
pub async fn send_message(&self, content: &str) -> Result<Response>;

// Variables: snake_case, descriptive
let retry_count: u32 = 3;

// Constants: UPPER_SNAKE_CASE
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 30;

// Modules: snake_case, file = module name
// src/llm_provider.rs → mod llm_provider;
```

## 🔄 Async/Await Best Practices
```rust
// ✅ DO: Use tokio::sync primitives for shared state
use tokio::sync::Mutex;
struct Agent {
    state: Arc<Mutex<AgentState>>,
}

// ✅ DO: Yield in long-running loops
for item in large_collection {
    process(item);
    tokio::task::yield_now().await; // Prevent starvation
}

// ❌ AVOID: Holding std::sync::Mutex across .await
// This can deadlock the runtime
async fn bad_example(lock: std::sync::Mutex<Data>) {
    let mut data = lock.lock().unwrap(); // BAD
    data.update();
    drop(data);
    some_async_call().await; // Runtime blocked until lock dropped
}

// ✅ DO: Use timeout for all external I/O
use tokio::time::{timeout, Duration};
let result = timeout(
    Duration::from_secs(10),
    client.post(url).send()
).await??;
```

## 🧱 Error Handling Strategy
```rust
// In library code (src/lib.rs or modules):
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("LLM API error: {0}")]
    LLM(#[from] reqwest::Error),
    
    #[error("Invalid tool call: {tool_name}")]
    InvalidTool { tool_name: String },
    
    #[error("Session not found: {id}")]
    SessionNotFound { id: uuid::Uuid },
}

pub type Result<T> = std::result::Result<T, AgentError>;

// In binary/application entry (src/main.rs):
use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let config = load_config()
        .context("Failed to load configuration")?;
    
    // Use ? for propagation, anyhow for context
    Ok(())
}
```

## 🧪 Testing Patterns
```rust
// Unit test in same file as implementation
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_message_serialization() {
        let msg = Message::user("Hello");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Hello"));
    }
    
    #[tokio::test]
    async fn test_async_operation() {
        let result = async_function().await;
        assert_eq!(result, Expected::Success);
    }
}

// Integration test in tests/ directory
// tests/chat_loop.rs
use smith_rust::{Agent, Config};

#[tokio::test]
async fn test_full_chat_cycle() {
    let agent = Agent::with_mock_provider();
    let response = agent.chat("Test message").await.unwrap();
    assert!(!response.content.is_empty());
}

// Mocking external dependencies
#[cfg(test)]
use mockall::{automock, predicate::*};

#[automock]
#[async_trait::async_trait]
pub trait LLMProvider {
    async fn chat(&self, messages: Vec<Message>) -> Result<LLMResponse>;
}
```

## 📦 Module Organization
```
src/
├── main.rs                 # Binary entry point (thin wrapper)
├── lib.rs                  # Library root, re-exports public API
├── config.rs               # Configuration loading & validation
├── domain/                 # Core business entities (pure Rust, no I/O)
│   ├── message.rs          # Message, Role, Content types
│   ├── tool.rs             # Tool trait & implementations
│   └── session.rs          # Session state & persistence interface
├── application/            # Use cases & orchestration
│   ├── chat_loop.rs        # Main agent loop logic
│   ├── tool_executor.rs    # Tool dispatch & result handling
│   └── planner.rs          # Multi-step reasoning (if applicable)
├── infrastructure/         # External integrations
│   ├── llm/
│   │   ├── provider.rs     # LLMProvider trait definition
│   │   ├── openai.rs       # OpenAI implementation
│   │   └── mock.rs         # Mock provider for testing
│   ├── storage/
│   │   ├── trait.rs        # Storage trait
│   │   ├── json_file.rs    # JSON file backend
│   │   └── postgres.rs     # PostgreSQL backend (feature-gated)
│   └── http_client.rs      # Shared HTTP client configuration
├── presentation/           # User-facing interfaces
│   ├── cli.rs              # clap-based CLI parser
│   └── tui.rs              # ratatui interface (optional feature)
└── error.rs                # Centralized error definitions
```

## 🔧 Cargo.toml Best Practices
```toml
[package]
name = "smith-rust"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"  # Explicit MSRV
license = "MIT OR Apache-2.0"

[lib]
name = "smith_rust"
path = "src/lib.rs"

[[bin]]
name = "smith"
path = "src/main.rs"

[features]
default = ["runtime-tokio"]
runtime-tokio = ["dep:tokio"]
mock-llm = []  # Enable mock LLM provider for testing
postgres = ["dep:sqlx"]  # Optional PostgreSQL support

[dependencies]
# Core runtime
tokio = { version = "1.0", features = ["full"], optional = true }

# CLI & UI
clap = { version = "4.0", features = ["derive"] }
ratatui = { version = "0.24", optional = true }

# Async & HTTP
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
async-trait = "0.1"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Logging & tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Utilities
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

# Optional dependencies
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio"], optional = true }

[dev-dependencies]
rstest = "0.18"          # Test fixtures
mockall = "0.12"         # Mocking framework
proptest = "1.4"         # Property-based testing
tokio-test = "0.4"       # Async test utilities

[profile.release]
lto = "thin"             # Faster builds with good optimization
codegen-units = 1        # Better optimization at cost of build time
```

## 🚀 Performance Checklist
- [ ] Use `&str` instead of `String` in function parameters where ownership not required
- [ ] Pre-allocate vectors with `Vec::with_capacity()` when size is known
- [ ] Use `Cow<'static, str>` for configuration values that are usually static
- [ ] Avoid `clone()` in loops — refactor to work with references
- [ ] Use `tokio::sync::broadcast` or `mpsc` for inter-task communication instead of shared state
- [ ] Profile with `cargo flamegraph` before optimizing prematurely