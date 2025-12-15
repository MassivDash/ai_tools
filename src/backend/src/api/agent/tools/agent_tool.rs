use crate::api::agent::types::{ToolCall, ToolCallResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

/// Tool metadata for registration and selection
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    pub id: String,
    pub name: String,
}

/// Trait that all tools must implement
#[async_trait]
pub trait AgentTool: Send + Sync {
    /// Get the tool's metadata
    fn metadata(&self) -> &ToolMetadata;

    /// Get the function definition for OpenAI-compatible API
    fn get_function_definition(&self) -> Value;

    /// Execute a tool call
    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult>;

    /// Check if this tool is available/ready to use
    fn is_available(&self) -> bool {
        true
    }
}
