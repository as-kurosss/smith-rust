//! Реестр инструментов — thread-safe хранилище для dispatch tool calls.
//!
//! Использует `DashMap` (при feature = "memory") или `RwLock<HashMap>` для параллельного доступа.

use std::sync::Arc;

#[cfg(feature = "memory")]
use dashmap::DashMap;
#[cfg(not(feature = "memory"))]
use std::collections::HashMap;
#[cfg(not(feature = "memory"))]
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::domain::tool::{Tool, ToolOutput};
use crate::error::{Result, SmithError};

use serde_json::json;

#[cfg(feature = "memory")]
type ToolMap = DashMap<String, Arc<dyn Tool>>;
#[cfg(not(feature = "memory"))]
type ToolMap = RwLock<HashMap<String, Arc<dyn Tool>>>;

/// Реестр инструментов.
///
/// Позволяет регистрировать инструменты по имени и выполнять их
/// через динамический dispatch.
pub struct ToolRegistry {
    /// Хранилище: имя → инструмент.
    tools: ToolMap,
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field(
                "backend",
                &if cfg!(feature = "memory") {
                    "DashMap"
                } else {
                    "RwLock<HashMap>"
                },
            )
            .finish_non_exhaustive()
    }
}

impl ToolRegistry {
    /// Создаёт пустой реестр.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tools: ToolMap::default(),
        }
    }

    /// Создаёт реестр с базовым набором инструментов.
    ///
    /// Включает: `echo`, `datetime`, `calculator`.
    ///
    /// # Note
    ///
    /// Для `memory` feature используется DashMap (sync register).
    /// Без `memory` — RwLock (async register).
    #[cfg(feature = "memory")]
    #[must_use]
    pub fn default_tools() -> Self {
        let registry = Self::new();
        registry.register_sync(Arc::new(crate::infrastructure::tools::echo::EchoTool::new()));
        registry.register_sync(Arc::new(
            crate::infrastructure::tools::datetime::DateTimeTool::new(),
        ));
        registry.register_sync(Arc::new(
            crate::infrastructure::tools::calculator::CalculatorTool::new(),
        ));

        // MemorySearchTool требует store + embedding provider.
        // Для default_tools используем mock-провайдер.
        use crate::infrastructure::tools::memory_search::MemorySearchTool;
        let store =
            Arc::new(crate::infrastructure::memory::json_store::JsonMemoryStore::new("./memory"));
        // Mock embedding provider для тестов
        let provider = Arc::new(MockEmbeddingProvider);
        let tool = MemorySearchTool::new(store, provider, 3);
        registry.register_sync(Arc::new(tool));

        registry
    }

    /// Создаёт реестр с базовым набором инструментов (без memory feature).
    #[cfg(not(feature = "memory"))]
    pub async fn default_tools() -> Self {
        let registry = Self::new();
        registry
            .register(Arc::new(crate::infrastructure::tools::echo::EchoTool::new()))
            .await;
        registry
            .register(Arc::new(
                crate::infrastructure::tools::datetime::DateTimeTool::new(),
            ))
            .await;
        registry
            .register(Arc::new(
                crate::infrastructure::tools::calculator::CalculatorTool::new(),
            ))
            .await;
        registry
    }

    /// Регистрирует инструмент синхронно (только для DashMap backend).
    #[cfg(feature = "memory")]
    fn register_sync(&self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        info!(name, "registering tool");
        self.tools.insert(name, tool);
    }

    /// Регистрирует инструмент в реестре.
    ///
    /// Если инструмент с таким именем уже существует, он перезаписывается.
    pub async fn register(&self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        info!(name, "registering tool");
        #[cfg(feature = "memory")]
        self.tools.insert(name, tool);
        #[cfg(not(feature = "memory"))]
        self.tools.write().await.insert(name, tool);
    }

    /// Выполняет инструмент по имени с указанными параметрами.
    ///
    /// # Errors
    ///
    /// Возвращает [`SmithError::ToolNotFound`] если инструмент не найден,
    /// или ошибку выполнения из самого инструмента.
    pub async fn execute(&self, name: &str, params: serde_json::Value) -> Result<ToolOutput> {
        #[cfg(feature = "memory")]
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| SmithError::ToolNotFound(name.to_string()))?;
        #[cfg(not(feature = "memory"))]
        let tool = self
            .tools
            .read()
            .await
            .get(name)
            .cloned()
            .ok_or_else(|| SmithError::ToolNotFound(name.to_string()))?;

        debug!(name, params = %params, "executing tool");
        tool.execute(params).await
    }

    /// Возвращает список зарегистрированных инструментов.
    pub async fn list(&self) -> Vec<String> {
        #[cfg(feature = "memory")]
        {
            self.tools.iter().map(|e| e.key().clone()).collect()
        }
        #[cfg(not(feature = "memory"))]
        {
            self.tools.read().await.keys().cloned().collect()
        }
    }

    /// Проверяет наличие инструмента.
    pub async fn has(&self, name: &str) -> bool {
        #[cfg(feature = "memory")]
        {
            self.tools.contains_key(name)
        }
        #[cfg(not(feature = "memory"))]
        {
            self.tools.read().await.contains_key(name)
        }
    }

    /// Возвращает JSON-описание всех инструментов (для передачи в LLM).
    #[must_use]
    pub async fn tool_definitions(&self) -> serde_json::Value {
        #[cfg(feature = "memory")]
        {
            let definitions: Vec<serde_json::Value> = self
                .tools
                .iter()
                .map(|e| {
                    let tool = e.value();
                    json!({
                        "type": "function",
                        "function": {
                            "name": tool.name(),
                            "description": tool.description(),
                            "parameters": tool.schema()
                        }
                    })
                })
                .collect();
            json!(definitions)
        }
        #[cfg(not(feature = "memory"))]
        {
            let map = self.tools.read().await;
            let definitions: Vec<serde_json::Value> = map
                .values()
                .map(|tool| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": tool.name(),
                            "description": tool.description(),
                            "parameters": tool.schema()
                        }
                    })
                })
                .collect();
            json!(definitions)
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_execute() {
        let registry = ToolRegistry::new();
        registry
            .register(Arc::new(crate::infrastructure::tools::echo::EchoTool::new()))
            .await;

        assert!(registry.has("echo").await);
        assert!(!registry.has("nonexistent").await);

        let output = registry
            .execute("echo", json!({"test": true}))
            .await
            .expect("execute should succeed");
        assert!(output.success);
    }

    #[tokio::test]
    async fn test_execute_nonexistent_tool() {
        let registry = ToolRegistry::new();
        let result = registry.execute("no_such_tool", json!({})).await;
        assert!(matches!(result, Err(SmithError::ToolNotFound(_))));
    }

    #[tokio::test]
    async fn test_default_tools() {
        let registry = ToolRegistry::default_tools();
        let tools = registry.list().await;
        assert!(tools.contains(&"echo".to_string()));
        assert!(tools.contains(&"datetime".to_string()));
        assert!(tools.contains(&"calculator".to_string()));
    }

    #[tokio::test]
    async fn test_tool_definitions() {
        let registry = ToolRegistry::default_tools();
        let defs = registry.tool_definitions().await;
        let arr = defs.as_array().expect("should be array");
        #[cfg(feature = "memory")]
        assert_eq!(arr.len(), 4); // echo, datetime, calculator, memory_search
        #[cfg(not(feature = "memory"))]
        assert_eq!(arr.len(), 3);
    }
}

// ===================== Mock Embedding Provider =====================

#[cfg(feature = "memory")]
struct MockEmbeddingProvider;

#[cfg(feature = "memory")]
#[async_trait::async_trait]
impl crate::domain::embedding::EmbeddingProvider for MockEmbeddingProvider {
    async fn embed(&self, _text: &str) -> crate::error::Result<Vec<f32>> {
        Ok(vec![0.0; 3])
    }

    fn dimension(&self) -> usize {
        3
    }
}
