use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::agent::tools::social::facebook_common::FacebookCredentials;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

const FIELDS: &str = "id,message,from,created_time";

/// Facebook Read Comments Tool implementation
/// Lists comments left by users on a given Page post (requires the
/// `pages_read_user_content` permission).
pub struct FacebookReadCommentsTool {
    metadata: ToolMetadata,
    credentials: FacebookCredentials,
}

impl FacebookReadCommentsTool {
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                id: "facebook_read_comments".to_string(),
                name: "Facebook Read Comments".to_string(),
                description: "Read comments users have left on a Facebook Page post".to_string(),
                category: ToolCategory::Social,
                tool_type: ToolType::FacebookCommentsRead,
            },
            credentials: FacebookCredentials::from_env(),
        }
    }

    async fn read_comments(&self, post_id: &str, limit: u32) -> Result<String> {
        let access_token = self.credentials.access_token()?;

        let client = Client::new();
        let url = self.credentials.graph_url(&format!("{}/comments", post_id));
        let limit_str = limit.to_string();

        let response = client
            .get(&url)
            .query(&[
                ("fields", FIELDS),
                ("limit", &limit_str),
                ("access_token", access_token),
            ])
            .send()
            .await
            .context("Failed to connect to Facebook Graph API")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Failed to read Facebook comments: {}",
                error_text
            ));
        }

        let data: serde_json::Value = response.json().await?;
        let comments = data["data"].as_array().cloned().unwrap_or_default();

        if comments.is_empty() {
            return Ok("No comments found on this post.".to_string());
        }

        Ok(comments
            .iter()
            .map(format_comment_line)
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

/// Renders a single Graph API comment object as a one-line summary.
fn format_comment_line(comment: &serde_json::Value) -> String {
    let id = comment["id"].as_str().unwrap_or("unknown");
    let from = comment["from"]["name"].as_str().unwrap_or("Unknown user");
    let message = comment["message"].as_str().unwrap_or("(no text)");
    let created = comment["created_time"].as_str().unwrap_or("unknown time");
    format!("[{}] {}: {} | id: {}", created, from, message, id)
}

#[async_trait]
impl AgentTool for FacebookReadCommentsTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "facebook_read_comments",
            "description": "Read comments users have left on a specific Facebook Page post.",
            "parameters": {
                "type": "object",
                "properties": {
                    "post_id": {
                        "type": "string",
                        "description": "The ID of the post to read comments from (as returned by facebook_read_posts or facebook_post)."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of comments to return (default 10, max 25)."
                    }
                },
                "required": ["post_id"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse tool call arguments")?;

        let post_id = args
            .get("post_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: post_id"))?;
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n.clamp(1, 25) as u32)
            .unwrap_or(10);

        println!("📘 Reading Facebook comments...");
        let result = self.read_comments(post_id, limit).await?;

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "facebook_read_comments".to_string(),
            result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_comment_with_full_data() {
        let comment = json!({
            "id": "c1",
            "from": {"name": "Jane Doe"},
            "message": "Great post!",
            "created_time": "2026-07-20T10:00:00+0000"
        });
        let line = format_comment_line(&comment);
        assert!(line.contains("Jane Doe"));
        assert!(line.contains("Great post!"));
        assert!(line.contains("c1"));
    }

    #[test]
    fn formats_comment_with_missing_author() {
        let comment = json!({"id": "c2", "message": "hi"});
        let line = format_comment_line(&comment);
        assert!(line.contains("Unknown user"));
    }
}
