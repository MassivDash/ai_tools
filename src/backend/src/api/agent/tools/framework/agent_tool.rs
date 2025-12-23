use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tool category with associated Material Icon name
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    /// Web/Internet related tools (web icon)
    Web,
    /// Financial/Money related tools (currency-usd icon)
    Financial,
    /// Database/Storage related tools (database icon)
    Database,
    /// Search/Query related tools (magnify icon)
    Search,
    /// File operations (file-document icon)
    File,
    /// Communication tools (send icon)
    Communication,
    /// Development/Code tools (code-tags icon)
    Development,
    /// General/Utility tools (wrench icon)
    Utility,
}

impl ToolCategory {
    /// Get the Material Icon name for this category
    pub fn icon_name(&self) -> &'static str {
        match self {
            ToolCategory::Web => "web",
            ToolCategory::Financial => "currency-usd",
            ToolCategory::Database => "database",
            ToolCategory::Search => "magnify",
            ToolCategory::File => "file-document",
            ToolCategory::Communication => "send",
            ToolCategory::Development => "code-tags",
            ToolCategory::Utility => "wrench",
        }
    }
}

/// Tool metadata for registration and selection
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    pub id: String,
    pub name: String,
    pub tool_type: ToolType,
    pub description: String,
    pub category: ToolCategory,
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
