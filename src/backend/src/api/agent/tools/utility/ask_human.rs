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
            "description": "Ask the user a multiple-choice question and pause execution until they select an option. Use this when you need explicit approval, selection, or input from the user before proceeding. IMPORTANT: You must output valid JSON. Do not include trailing commas. Ensure all strings are properly escaped. You must ALWAYS include an option named exactly 'Other' as the final choice in the options array.",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::core::types::FunctionCall;

    #[test]
    fn metadata_and_function_definition_describe_the_ask_human_tool() {
        let tool = AskHumanTool::new();
        assert_eq!(tool.metadata().id, "ask_human");
        assert_eq!(tool.metadata().name, "Ask Human");
        assert_eq!(tool.metadata().category, ToolCategory::Utility);
        assert_eq!(tool.metadata().tool_type, ToolType::AskHuman);
        // Nothing external is needed, so it is always offered.
        assert!(tool.is_available());

        let def = tool.get_function_definition();
        assert_eq!(def["name"], "ask_human");
        assert_eq!(
            def["parameters"]["required"],
            json!(["question", "options"])
        );
        assert_eq!(def["parameters"]["properties"]["options"]["type"], "array");
        assert_eq!(
            def["parameters"]["properties"]["options"]["items"]["type"],
            "string"
        );
        // The 'Other' requirement lives in the description the LLM sees.
        assert!(def["description"]
            .as_str()
            .expect("a description")
            .contains("'Other' as the final choice"));
    }

    #[tokio::test]
    async fn executing_it_directly_is_an_error_because_the_loop_must_intercept_it() {
        let tool = AskHumanTool::new();
        let call = ToolCall {
            id: "call_ask".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "ask_human".to_string(),
                arguments: r#"{"question": "Proceed?", "options": ["Yes", "Other"]}"#.to_string(),
            },
        };

        let error = tool
            .execute(&call)
            .await
            .expect_err("Running this tool for real must never succeed");

        assert_eq!(
            error.to_string(),
            "This tool requires human input and should be intercepted by the streaming loop."
        );
    }
}
