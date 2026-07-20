use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use std::env;

/// Bluesky Post Tool implementation
/// Allows the agent to post to Bluesky
pub struct BlueskyPostTool {
    metadata: ToolMetadata,
    handle: Option<String>,
    password: Option<String>,
}

impl BlueskyPostTool {
    /// Create a new Bluesky tool
    pub fn new() -> Self {
        let handle = env::var("BLUESKY_HANDLE").ok();
        let password = env::var("BLUESKY_PASSWORD").ok();

        Self {
            metadata: ToolMetadata {
                id: "bluesky_post".to_string(),
                name: "Bluesky Post".to_string(),
                description: "Post a message to the user's Bluesky account (Max 300 characters)"
                    .to_string(),
                category: ToolCategory::Social,
                tool_type: ToolType::BlueskyPost,
            },
            handle,
            password,
        }
    }

    /// Authenticate and post to Bluesky
    async fn post_to_bluesky(&self, text: &str) -> Result<String> {
        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("BLUESKY_HANDLE environment variable not set"))?;
        let password = self
            .password
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("BLUESKY_PASSWORD environment variable not set"))?;

        let client = Client::new();

        // 1. Create session
        let session_response = client
            .post("https://bsky.social/xrpc/com.atproto.server.createSession")
            .json(&json!({
                "identifier": handle,
                "password": password
            }))
            .send()
            .await
            .context("Failed to connect to Bluesky API to create session")?;

        if !session_response.status().is_success() {
            let error_text = session_response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Failed to authenticate with Bluesky: {}",
                error_text
            ));
        }

        let session_data: serde_json::Value = session_response.json().await?;
        let access_token = session_data["accessJwt"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid session response: missing accessJwt"))?;
        let did = session_data["did"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid session response: missing did"))?;

        // 2. Create post record
        let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let record = json!({
            "repo": did,
            "collection": "app.bsky.feed.post",
            "record": {
                "$type": "app.bsky.feed.post",
                "text": text,
                "createdAt": now
            }
        });

        let post_response = client
            .post("https://bsky.social/xrpc/com.atproto.repo.createRecord")
            .header("Authorization", format!("Bearer {}", access_token))
            .json(&record)
            .send()
            .await
            .context("Failed to post record to Bluesky")?;

        if !post_response.status().is_success() {
            let error_text = post_response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Failed to post to Bluesky: {}", error_text));
        }

        Ok(format!("Successfully posted to Bluesky: '{}'", text))
    }
}

#[async_trait]
impl AgentTool for BlueskyPostTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "bluesky_post",
            "description": "Post a text message to Bluesky. IMPORTANT: The text MUST be 300 characters or less. Do not exceed this limit.",
            "parameters": {
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "The text content of the post to create. STRICT LIMIT: Maximum 300 characters."
                    }
                },
                "required": ["text"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse tool call arguments")?;

        let text = args
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: text"))?;

        if text.chars().count() > 300 {
            return Err(anyhow::anyhow!(
                "Post text exceeds Bluesky's 300 character limit"
            ));
        }

        println!("🦋 Posting to Bluesky...");
        let result = self.post_to_bluesky(text).await?;
        println!("✅ {}", result);

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "bluesky_post".to_string(),
            result,
        })
    }
}
