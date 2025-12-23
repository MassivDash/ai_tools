use super::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use super::registry::ToolRegistry;
use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

// Mock Tool Implementation
struct MockTool {
    available: bool,
    metadata: ToolMetadata,
}

impl MockTool {
    fn new(id: &str, available: bool) -> Self {
        Self {
            available,
            metadata: ToolMetadata {
                id: id.to_string(),
                name: format!("mock_function_{}", id),
                description: "A mock tool for testing".to_string(),
                category: ToolCategory::Utility,
                tool_type: ToolType::Weather, // Using an existing variant
            },
        }
    }
}

#[async_trait]
impl AgentTool for MockTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": self.metadata.name,
            "description": "A mock function",
            "parameters": {
                "type": "object",
                "properties": {}
            }
        })
    }

    async fn execute(&self, _tool_call: &ToolCall) -> anyhow::Result<ToolCallResult> {
        Ok(ToolCallResult {
            tool_name: "mock_tool".to_string(),
            result: "Executed".to_string(),
        })
    }

    fn is_available(&self) -> bool {
        self.available
    }
}

#[test]
fn test_registry_add_retrieve() {
    let mut registry = ToolRegistry::new();
    let tool = Arc::new(MockTool::new("tool1", true));

    // Register
    registry
        .register(tool.clone())
        .expect("Failed to register tool");

    // Retrieve by ID
    let retrieved = registry.get_tool("tool1");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().metadata().id, "tool1");

    // Retrieve by Name
    let retrieved_by_name = registry.get_tool_by_name("mock_function_tool1");
    assert!(retrieved_by_name.is_some());
}

#[test]
fn test_registry_duplicate_id() {
    let mut registry = ToolRegistry::new();
    let tool1 = Arc::new(MockTool::new("tool1", true));
    let tool2 = Arc::new(MockTool::new("tool1", true));

    // First registration ok
    registry
        .register(tool1)
        .expect("Failed to register first tool");

    // Duplicate registration should fail
    assert!(registry.register(tool2).is_err());
}

#[test]
fn test_registry_unavailable_tool() {
    let mut registry = ToolRegistry::new();
    let tool = Arc::new(MockTool::new("tool_unavailable", false));

    // Registration should succeed (return Ok) but NOT add the tool
    registry
        .register(tool)
        .expect("Failed to register unavailable tool");

    // Verify it's not in the registry
    assert!(registry.get_tool("tool_unavailable").is_none());
}

#[test]
fn test_get_all_tools() {
    let mut registry = ToolRegistry::new();
    registry
        .register(Arc::new(MockTool::new("t1", true)))
        .unwrap();
    registry
        .register(Arc::new(MockTool::new("t2", true)))
        .unwrap();

    let tools = registry.get_all_tools();
    assert_eq!(tools.len(), 2);
}
