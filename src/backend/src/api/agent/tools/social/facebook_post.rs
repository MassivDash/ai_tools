use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::agent::tools::social::facebook_common::FacebookCredentials;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

/// Facebook Page Post Tool implementation
/// Allows the agent to post to the user's Facebook Page via the Graph API
/// (requires the `pages_manage_posts` permission).
pub struct FacebookPostTool {
    metadata: ToolMetadata,
    credentials: FacebookCredentials,
}

impl FacebookPostTool {
    /// Create a new Facebook tool
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                id: "facebook_post".to_string(),
                name: "Facebook Post".to_string(),
                description: "Post a message to the user's Facebook Page".to_string(),
                category: ToolCategory::Social,
                tool_type: ToolType::FacebookPost,
            },
            credentials: FacebookCredentials::from_env(),
        }
    }

    /// Post to the configured Facebook Page via the Graph API
    async fn post_to_facebook(&self, message: &str, link: Option<&str>) -> Result<String> {
        let page_id = self.credentials.page_id()?;
        let access_token = self.credentials.access_token()?;

        let client = Client::new();
        let url = self.credentials.graph_url(&format!("{}/feed", page_id));

        let mut params: Vec<(&str, &str)> =
            vec![("message", message), ("access_token", access_token)];
        if let Some(link) = link {
            params.push(("link", link));
        }

        let response = client
            .post(&url)
            .form(&params)
            .send()
            .await
            .context("Failed to connect to Facebook Graph API")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Failed to post to Facebook: {}",
                error_text
            ));
        }

        let data: serde_json::Value = response.json().await?;
        let post_id = data["id"].as_str().unwrap_or("unknown");

        Ok(format!(
            "Successfully posted to Facebook Page (post id: {})",
            post_id
        ))
    }
}

#[async_trait]
impl AgentTool for FacebookPostTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "facebook_post",
            "description": "Post a text message to the user's Facebook Page. Optionally attach a link, which Facebook will render as a preview card.",
            "parameters": {
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "The text content of the post to create."
                    },
                    "link": {
                        "type": "string",
                        "description": "Optional URL to attach to the post. Facebook renders it as a link preview card."
                    }
                },
                "required": ["message"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse tool call arguments")?;

        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: message"))?;
        let link = args.get("link").and_then(|v| v.as_str());

        println!("📘 Posting to Facebook...");
        let result = self.post_to_facebook(message, link).await?;
        println!("✅ {}", result);

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "facebook_post".to_string(),
            result,
        })
    }
}
