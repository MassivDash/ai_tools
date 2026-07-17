use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct AskHumanTool {
    metadata: ToolMetadata,
}

impl AskHumanTool {
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                id: "ask_human".to_string(),
                name: "Ask Human".to_string(),
                tool_type: ToolType::AskHuman,
                description: "Ask the user a multiple-choice question and pause execution until they select an option.".to_string(),
                category: ToolCategory::Utility,
            },
        }
    }
}

#[async_trait]
impl AgentTool for AskHumanTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "ask_human",
            "description": "Ask the user a multiple-choice question and pause execution until they select an option. Use this when you need explicit approval, selection, or input from the user before proceeding. You must ALWAYS include an 'Other' option so the user can provide custom input.",
            "parameters": {
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "The question or prompt to display to the user."
                    },
                    "options": {
                        "type": "array",
                        "description": "A list of strings representing the options the user can select. You MUST always include an option named exactly 'Other' as the final choice.",
                        "items": {
                            "type": "string"
                        }
                    }
                },
                "required": ["question", "options"]
            }
        })
    }

    async fn execute(&self, _tool_call: &ToolCall) -> Result<ToolCallResult> {
        // This tool should never be executed normally.
        // The streaming loop will intercept it and pause execution.
        anyhow::bail!(
            "This tool requires human input and should be intercepted by the streaming loop."
        );
    }
}
