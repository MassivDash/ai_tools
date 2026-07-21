use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::env;

/// Default Graph API version, used when FACEBOOK_GRAPH_API_VERSION isn't
/// set. Meta sunsets versions on a ~2-year cycle; overriding via env var
/// avoids needing a code change/redeploy when this one is retired.
const DEFAULT_GRAPH_API_VERSION: &str = "v21.0";

/// Facebook Page Post Tool implementation
/// Allows the agent to post to the user's Facebook Page via the Graph API
pub struct FacebookPostTool {
    metadata: ToolMetadata,
    page_id: Option<String>,
    access_token: Option<String>,
    graph_api_version: String,
}

impl FacebookPostTool {
    /// Create a new Facebook tool
    pub fn new() -> Self {
        let page_id = env::var("FACEBOOK_PAGE_ID").ok();
        let access_token = env::var("FACEBOOK_PAGE_ACCESS_TOKEN").ok();
        let graph_api_version = env::var("FACEBOOK_GRAPH_API_VERSION")
            .unwrap_or_else(|_| DEFAULT_GRAPH_API_VERSION.to_string());

        Self {
            metadata: ToolMetadata {
                id: "facebook_post".to_string(),
                name: "Facebook Post".to_string(),
                description: "Post a message to the user's Facebook Page".to_string(),
                category: ToolCategory::Social,
                tool_type: ToolType::FacebookPost,
            },
            page_id,
            access_token,
            graph_api_version,
        }
    }

    /// Post to the configured Facebook Page via the Graph API
    async fn post_to_facebook(&self, message: &str, link: Option<&str>) -> Result<String> {
        let page_id = self
            .page_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("FACEBOOK_PAGE_ID environment variable not set"))?;
        let access_token = self.access_token.as_ref().ok_or_else(|| {
            anyhow::anyhow!("FACEBOOK_PAGE_ACCESS_TOKEN environment variable not set")
        })?;

        let client = Client::new();
        let url = format!(
            "https://graph.facebook.com/{}/{}/feed",
            self.graph_api_version, page_id
        );

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
