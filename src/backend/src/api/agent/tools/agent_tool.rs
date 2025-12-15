use crate::api::agent::types::{ToolCall, ToolCallResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

/// Tool metadata for registration and selection
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub keywords: Vec<String>, // Keywords that help identify when to use this tool
    #[allow(dead_code)] // Part of extensible API for future categorization features
    pub category: ToolCategory,
}

/// Tool categories for better organization
/// Part of extensible API for future categorization and filtering features
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ToolCategory {
    Search,
    DataQuery,
    Analysis,
    Communication,
    Other,
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

    /// Get a relevance score for a given query (0.0 to 1.0)
    /// Higher score means the tool is more relevant to the query
    fn calculate_relevance(&self, query: &str) -> f64 {
        let query_lower = query.to_lowercase();
        let mut score = 0.0;
        let keyword_count = self.metadata().keywords.len() as f64;

        if keyword_count == 0.0 {
            return 0.5; // Default score if no keywords
        }

        for keyword in &self.metadata().keywords {
            let keyword_lower = keyword.to_lowercase();
            if query_lower.contains(&keyword_lower) {
                score += 1.0 / keyword_count;
            }
        }

        // Also check if description contains relevant terms
        let desc_lower = self.metadata().description.to_lowercase();
        if query_lower
            .split_whitespace()
            .any(|word| desc_lower.contains(word))
        {
            score += 0.2;
        }

        score.min(1.0)
    }
}
