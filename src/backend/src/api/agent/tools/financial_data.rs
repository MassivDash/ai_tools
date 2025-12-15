use crate::api::agent::tools::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::agent::types::{ToolCall, ToolCallResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

/// Financial Data tool implementation (mock)
pub struct FinancialDataTool {
    metadata: ToolMetadata,
}

impl FinancialDataTool {
    /// Create a new Financial Data tool
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                id: "2".to_string(),
                name: "financial sql query".to_string(),
                description:
                    "A sql data query that will result in providing latest user shopping info"
                        .to_string(),
                keywords: vec![
                    "financial".to_string(),
                    "purchase".to_string(),
                    "shopping".to_string(),
                    "transaction".to_string(),
                    "spending".to_string(),
                    "buy".to_string(),
                    "money".to_string(),
                    "cost".to_string(),
                    "price".to_string(),
                    "sql".to_string(),
                    "data".to_string(),
                ],
                category: ToolCategory::DataQuery,
            },
        }
    }
}

#[async_trait]
impl AgentTool for FinancialDataTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "get_financial_data",
            "description": "Get financial data including recent purchases and transactions. Use this when the user asks about their purchases, spending, last buys, or financial transactions.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query_type": {
                        "type": "string",
                        "description": "Type of financial data requested (e.g., 'purchases', 'transactions', 'spending')",
                        "enum": ["purchases", "transactions", "spending"]
                    }
                },
                "required": ["query_type"]
            }
        })
    }

    async fn execute(&self, _tool_call: &ToolCall) -> Result<ToolCallResult> {
        // Mock financial data
        let financial_data = r#"Recent Purchases:
1. TV - $200.00
2. Phone - $100.00
3. Candy bar - $5.00

Total: $305.00"#;

        Ok(ToolCallResult {
            tool_name: "get_financial_data".to_string(),
            result: financial_data.to_string(),
        })
    }
}
