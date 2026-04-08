# Prompt Templates for Qwen Code

## 🎯 Template: New Feature Implementation
```
@workspace /implement feature

CONTEXT: See AGENTS.md and RUST_GUIDELINES.md for project standards.

TASK: Implement {feature_name} for step {XX} in smith-rust.

REQUIREMENTS:
1. Add types/traits to src/domain/{module}.rs
2. Implement logic in src/application/{module}.rs  
3. Add infrastructure adapter if needed (src/infrastructure/{provider}.rs)
4. Update public API in src/lib.rs with re-exports
5. Add integration test in tests/{feature}_test.rs

CONSTRAINTS:
- Follow error handling pattern: thiserror for lib, anyhow for bin
- All async functions must use tokio runtime
- Add #[cfg(test)] module with at least 2 test cases
- Document public items with /// comments including example

OUTPUT FORMAT:
1. Brief design summary (3 bullets)
2. Code changes by file (use ```rust blocks)
3. cargo command to verify: `cargo test --features {feature}`
4. Next suggested step for tutorial progression
```

## 🐛 Template: Bug Fix / Refactor
```
@workspace /fix issue

CONTEXT: {brief description of problem}

CURRENT BEHAVIOR: {what happens now}
EXPECTED BEHAVIOR: {what should happen}

AFFECTED MODULES: [list files or components]

INVESTIGATION STEPS:
1. Reproduce with minimal test case in tests/regression/
2. Identify root cause (ownership? async race? type mismatch?)
3. Propose fix with before/after code snippet

FIX REQUIREMENTS:
- Maintain backward compatibility unless breaking change is intentional
- Add regression test to prevent recurrence
- Update documentation if API behavior changes
- Run `cargo clippy -- -D warnings` to ensure no new lints

VERIFICATION:
- [ ] cargo build --all-features
- [ ] cargo test --all-features  
- [ ] cargo clippy -- -D warnings
- [ ] cargo fmt --check
```

## 🧪 Template: Test Generation
```
@workspace /generate tests

TARGET: {module or function to test}

TEST STRATEGY:
1. Unit tests: Cover happy path + all error variants
2. Edge cases: Empty input, max size, concurrent calls
3. Property tests: Use proptest for functions with invariants
4. Integration: Test module interactions with mock dependencies

MOCKING APPROACH:
- Use mockall for trait-based dependencies
- Create test fixtures with rstest for reusable setup
- Isolate external I/O with feature flag "mock-llm"

EXAMPLE TEST STRUCTURE:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    
    #[rstest]
    #[case("input1", Expected::Success)]
    #[case("invalid", Expected::Error(_))]
    fn test_function_name(#[case] input: &str, #[case] expected: Expected) {
        // Arrange
        let mut mock = MockDependency::new();
        mock.expect_call()
            .with(eq(input))
            .returning(|_| Ok(MockResponse));
        
        // Act
        let result = function_under_test(input, &mock).await;
        
        // Assert
        assert_matches!(result, expected);
    }
}
```

OUTPUT: Provide complete test module ready to paste into target file.
```

## 🔄 Template: Step Migration (Python → Rust)
```
@workspace /port step

SOURCE: Python step {XX} from build-your-own-openclaw
TARGET: Rust equivalent in smith-rust step {XX}

MIGRATION CHECKLIST:
[ ] Identify core logic (separate from Python-specific libraries)
[ ] Map Python types to Rust equivalents (dict → HashMap, list → Vec)
[ ] Replace asyncio with tokio async/await patterns
[ ] Convert dynamic typing to explicit Rust types with serde
[ ] Implement error handling with thiserror instead of try/except

KEY TRANSFORMATIONS:
- Python `async def` → Rust `async fn` with `#[async_trait]` if in trait
- Python `dict` with dynamic keys → Rust `enum` + `serde(tag = "type")`
- Python `typing.Optional[T]` → Rust `Option<T>`
- Python `raise ValueError(...)` → Rust `Err(AgentError::ValidationError(...))`

ARCHITECTURE ADAPTATION:
- Preserve tutorial educational value: keep code readable, add comments
- Add Rust-specific learning notes: "Why we use Arc<Mutex<T>> here"
- Include cargo command to run this step independently

DELIVERABLE:
1. Side-by-side comparison: Python snippet → Rust equivalent
2. Full Rust implementation with tests
3. README snippet explaining Rust concepts introduced in this step
```
```