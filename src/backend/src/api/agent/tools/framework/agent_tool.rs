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
    /// Google Workspace tools (google icon)
    Google,
    /// Social media tools (share-variant icon)
    Social,
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
            ToolCategory::Google => "google",
            ToolCategory::Social => "share-variant",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_category_maps_to_its_material_icon() {
        // The frontend renders these names directly, so each one is part of the
        // contract rather than an implementation detail.
        let mappings = [
            (ToolCategory::Web, "web"),
            (ToolCategory::Financial, "currency-usd"),
            (ToolCategory::Database, "database"),
            (ToolCategory::Search, "magnify"),
            (ToolCategory::File, "file-document"),
            (ToolCategory::Google, "google"),
            (ToolCategory::Social, "share-variant"),
            (ToolCategory::Development, "code-tags"),
            (ToolCategory::Utility, "wrench"),
        ];
        for (category, icon) in mappings {
            assert_eq!(category.icon_name(), icon, "{:?}", category);
        }
    }

    #[test]
    fn categories_round_trip_through_snake_case_json() {
        assert_eq!(
            serde_json::to_value(ToolCategory::Development).unwrap(),
            serde_json::json!("development")
        );
        assert_eq!(
            serde_json::from_value::<ToolCategory>(serde_json::json!("file")).unwrap(),
            ToolCategory::File
        );
    }
}
