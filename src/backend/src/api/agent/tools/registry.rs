use crate::api::agent::tools::agent_tool::{AgentTool, ToolMetadata};
use crate::api::agent::types::{Tool, ToolCall, ToolCallResult};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;

/// Registry for managing all available tools
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn AgentTool>>,
    metadata_map: HashMap<String, ToolMetadata>,
}

impl ToolRegistry {
    /// Create a new empty tool registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            metadata_map: HashMap::new(),
        }
    }

    /// Register a tool in the registry
    pub fn register(&mut self, tool: Arc<dyn AgentTool>) -> Result<()> {
        let metadata = tool.metadata();
        let id = metadata.id.clone();
        let name = metadata.name.clone();

        // Check if tool is available before registering
        if !tool.is_available() {
            println!("⚠️ Tool '{}' is not available, skipping registration", name);
            return Ok(()); // Don't error, just skip unavailable tools
        }

        // Check for duplicate IDs
        if self.tools.contains_key(&id) {
            return Err(anyhow::anyhow!("Tool with ID '{}' already registered", id));
        }

        self.metadata_map.insert(id.clone(), metadata.clone());
        self.tools.insert(id, Arc::clone(&tool));

        println!("✅ Registered tool: {} ({})", name, metadata.id);
        Ok(())
    }

    /// Get a tool by ID
    /// Used internally by execute_tool_call and for tool verification
    pub fn get_tool(&self, tool_id: &str) -> Option<Arc<dyn AgentTool>> {
        self.tools.get(tool_id).map(Arc::clone)
    }

    /// Get a tool by name (function name from get_function_definition)
    pub fn get_tool_by_name(&self, name: &str) -> Option<Arc<dyn AgentTool>> {
        self.tools
            .values()
            .find(|tool| {
                // Check both metadata name and function definition name
                let func_def = tool.get_function_definition();
                let func_name = func_def.get("name").and_then(|v| v.as_str()).unwrap_or("");
                tool.metadata().name == name || func_name == name
            })
            .map(Arc::clone)
    }

    /// Get all registered tool IDs
    pub fn get_all_tool_ids(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get all registered tools
    pub fn get_all_tools(&self) -> Vec<Arc<dyn AgentTool>> {
        self.tools.values().map(Arc::clone).collect()
    }

    /// Build OpenAI-compatible tool definitions for all registered tools
    pub fn build_tool_definitions(&self) -> Result<Vec<Tool>> {
        let mut definitions = Vec::new();

        for tool in self.tools.values() {
            let function_def = tool.get_function_definition();
            let function: crate::api::agent::types::FunctionDefinition =
                serde_json::from_value(function_def.clone())
                    .context("Failed to parse function definition")?;

            definitions.push(Tool {
                tool_type: "function".to_string(),
                function,
            });
        }

        Ok(definitions)
    }

    /// Execute a tool call by function name
    pub async fn execute_tool_call(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        println!("🔍 Looking for tool: '{}'", tool_call.function.name);
        println!(
            "🔍 Available tools in registry: {:?}",
            self.tools.keys().collect::<Vec<_>>()
        );

        // Try to find tool by function name
        let tool = self
            .get_tool_by_name(&tool_call.function.name)
            .ok_or_else(|| {
                let available_names: Vec<String> = self
                    .tools
                    .values()
                    .map(|t| t.metadata().name.clone())
                    .collect();
                anyhow::anyhow!(
                    "Tool '{}' not found. Available tools: {:?}",
                    tool_call.function.name,
                    available_names
                )
            })?;

        println!("✅ Found tool: {}", tool.metadata().name);
        tool.execute(tool_call).await
    }

    /// Check if a tool is registered
    pub fn is_registered(&self, tool_id: &str) -> bool {
        self.tools.contains_key(tool_id)
    }

    /// Get the number of registered tools
    pub fn count(&self) -> usize {
        self.tools.len()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
