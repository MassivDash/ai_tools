use crate::api::agent::types::{ToolCall, ToolCallResult};
use anyhow::Result;
use serde_json::json;

/// Financial Data tool implementation (mock)
pub struct FinancialDataTool;

impl FinancialDataTool {
    /// Get the function definition for OpenAI-compatible API
    pub fn get_function_definition() -> serde_json::Value {
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

    /// Execute a tool call
    pub async fn execute_tool_call(_tool_call: &ToolCall) -> Result<ToolCallResult> {
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
