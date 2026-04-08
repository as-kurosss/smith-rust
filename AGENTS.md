# AGENT INSTRUCTIONS: smith-rust

## 🎯 Project Goal
Reimplement `build-your-own-openclaw` (Python) in Rust as a production-ready, tutorial-style AI agent framework. Preserve the 18-step educational structure while applying Rust best practices. The project will be published as `smith-rust`.

## ⚙️ Technical Stack (MANDATORY)
| Component | Crate/Tool | Version Constraint | Notes |
|-----------|-----------|-------------------|-------|
| Runtime | `tokio` | `1.0`, features: `["full"]` | Single-threaded or multi-threaded based on config |
| CLI | `clap` | `4.0`, features: `["derive"]` | Use `#[derive(Parser)]` pattern |
| HTTP Client | `reqwest` | `0.12`, features: `["json", "rustls-tls"]` | Avoid native TLS for portability |
| Serialization | `serde`, `serde_json`, `serde_yaml` | `1.0` | Always derive `Serialize`, `Deserialize` |
| Error Handling | `thiserror` (lib), `anyhow` (bin) | `1.0` | See `RUST_GUIDELINES.md` |
| Logging | `tracing`, `tracing-subscriber` | `0.1` | Structured logging only, no `println!` |
| Testing | `rstest` (fixtures), `mockall` (mocks) | Latest | See `RUST_GUIDELINES.md` |
| Async Traits | `async-trait` | `0.1` | For trait objects with async methods |

## 🚫 Absolute Prohibitions
1. **NO `unwrap()` / `expect()`** in library code — use `?` operator or proper error propagation.
2. **NO `println!` / `eprintln!`** — use `tracing::info!`, `tracing::error!`, etc.
3. **NO blocking calls in async context** — use `tokio::task::spawn_blocking` for CPU-bound work.
4. **NO `std::sync::Mutex` across `.await`** — use `tokio::sync::Mutex` or `RwLock` for async-shared state.
5. **NO unbounded channels** — always specify capacity or use `mpsc::channel` with explicit limits.
6. **NO `String` cloning in hot paths** — prefer `&str`, `Cow<'static, str>`, or `Arc<str>` where applicable.

## ✅ Required Patterns
1. **Newtype pattern** for domain types (e.g., `struct ApiKey(String)`).
2. **Builder pattern** for complex configuration structs.
3. **Trait-based abstraction** for external dependencies (LLM providers, tools, storage).
4. **Feature flags** for optional functionality (`features = ["mock-llm", "postgres"]`).
5. **Explicit lifetimes** where compiler cannot infer — document with `// SAFETY:` if using `unsafe`.

## 📐 Architecture Principles
1. **Layered architecture**: `domain` → `application` → `infrastructure` → `presentation`.
2. **Dependency inversion**: High-level modules depend on abstractions, not concrete implementations.
3. **Single responsibility per module**: One file = one coherent concept.
4. **Immutable by default**: Use `&T` and `Cow` before reaching for `Mutex`.
5. **Async at boundaries only**: Keep business logic sync where possible; wrap async I/O at edges.

## 🔄 Development Workflow for Agent
When generating code, ALWAYS follow this sequence:
1. **Analyze**: Briefly restate the requirement and identify affected modules.
2. **Design**: List types, traits, and functions to add/modify (3-5 bullet points).
3. **Implement**: Generate code with full type annotations and documentation comments (`///`).
4. **Test**: Provide at least one `#[cfg(test)]` module with a meaningful test case.
5. **Verify**: Suggest the exact `cargo` command to compile/test the change.

## 📝 Commit & Documentation Standards
- All public items must have `///` doc comments with examples where non-trivial.
- Use `// TODO:` with GitHub issue reference for incomplete features.
- Commit messages: `feat(step03): add session persistence with JSON backend`.
- Each tutorial step must be independently compilable via `cargo build --features stepXX`.

## 🧪 Testing Requirements
1. **Unit tests**: Cover error paths and edge cases for each public function.
2. **Integration tests**: In `tests/` directory, test module interactions.
3. **Mocking**: Use `mockall` for external dependencies; never call real APIs in tests.
4. **Property-based**: Use `proptest` for functions with complex input invariants.
5. **Async tests**: Always use `#[tokio::test]` with explicit runtime configuration.

## 🛡️ Security & Reliability
1. **Secrets management**: Never hardcode API keys; use `std::env::var` or secret managers.
2. **Input validation**: Sanitize all user input before processing or logging.
3. **Rate limiting**: Implement token bucket or leaky bucket for external API calls.
4. **Graceful shutdown**: Handle `SIGINT`/`SIGTERM` with `tokio::signal` and cleanup hooks.
5. **Resource limits**: Set timeouts on all async operations (`tokio::time::timeout`).

## 🛡️ Doc Comment Rules (MANDATORY)
- In `mod.rs` files: ALWAYS use `//!` for module-level documentation.
- NEVER leave an empty line between `///` and the item it documents.
- If a comment block ends with an empty line, ensure it's intentional and use `//!` for file-level docs.
- Run `cargo clippy -- -D clippy::empty_line_after_doc_comments` before finalizing code generation.

## 🤖 Automation Protocol

When a step is complete and all checks pass, use the verification script:

```powershell
./scripts/verify-and-publish.ps1 -Step "XX" -Message "краткое описание" -Features "openai postgres"
```

For local testing without push: add `--NoPush` flag.

NEVER attempt to commit/push without running verification first.

## 📚 Reference Materials
- Original Python project: https://github.com/czl9707/build-your-own-openclaw
- Rust Async Book: https://rust-lang.github.io/async-book/
- Rust API Guidelines: https://rust-lang.github.io/api-guidelines/
- Tokio Tutorial: https://tokio.rs/tokio/tutorial