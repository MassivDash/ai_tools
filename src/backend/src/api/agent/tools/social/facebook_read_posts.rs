use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::agent::tools::social::facebook_common::FacebookCredentials;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

const FIELDS: &str = "id,message,created_time,permalink_url,likes.summary(true),comments.summary(true),shares";

/// Facebook Read Posts Tool implementation
/// Lists recent posts on the user's Facebook Page with engagement counts
/// (requires the `pages_read_engagement` permission).
pub struct FacebookReadPostsTool {
    metadata: ToolMetadata,
    credentials: FacebookCredentials,
}

impl FacebookReadPostsTool {
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                id: "facebook_read_posts".to_string(),
                name: "Facebook Read Posts".to_string(),
                description: "Read recent posts from the user's Facebook Page, including like/comment/share counts".to_string(),
                category: ToolCategory::Social,
                tool_type: ToolType::FacebookPostsRead,
            },
            credentials: FacebookCredentials::from_env(),
        }
    }

    async fn read_posts(&self, limit: u32) -> Result<String> {
        let page_id = self.credentials.page_id()?;
        let access_token = self.credentials.access_token()?;

        let client = Client::new();
        let url = self.credentials.graph_url(&format!("{}/posts", page_id));
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
                "Failed to read Facebook posts: {}",
                error_text
            ));
        }

        let data: serde_json::Value = response.json().await?;
        let posts = data["data"].as_array().cloned().unwrap_or_default();

        if posts.is_empty() {
            return Ok("No posts found on the Page.".to_string());
        }

        Ok(posts.iter().map(format_post_summary).collect::<Vec<_>>().join("\n"))
    }
}

/// Renders a single Graph API post object as a one-line summary.
fn format_post_summary(post: &serde_json::Value) -> String {
    let id = post["id"].as_str().unwrap_or("unknown");
    let message = post["message"].as_str().unwrap_or("(no text)");
    let created = post["created_time"].as_str().unwrap_or("unknown time");
    let likes = post["likes"]["summary"]["total_count"].as_u64().unwrap_or(0);
    let comments = post["comments"]["summary"]["total_count"].as_u64().unwrap_or(0);
    let shares = post["shares"]["count"].as_u64().unwrap_or(0);
    format!(
        "[{}] {} | likes: {} comments: {} shares: {} | id: {}",
        created, message, likes, comments, shares, id
    )
}

#[async_trait]
impl AgentTool for FacebookReadPostsTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "facebook_read_posts",
            "description": "Read recent posts from the user's Facebook Page, including like, comment, and share counts.",
            "parameters": {
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of posts to return (default 10, max 25)."
                    }
                },
                "required": []
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse tool call arguments")?;

        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n.clamp(1, 25) as u32)
            .unwrap_or(10);

        println!("📘 Reading Facebook posts...");
        let result = self.read_posts(limit).await?;

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "facebook_read_posts".to_string(),
            result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_post_with_full_engagement_data() {
        let post = json!({
            "id": "123_456",
            "message": "Hello world",
            "created_time": "2026-07-20T10:00:00+0000",
            "likes": {"summary": {"total_count": 5}},
            "comments": {"summary": {"total_count": 2}},
            "shares": {"count": 1}
        });
        let line = format_post_summary(&post);
        assert!(line.contains("Hello world"));
        assert!(line.contains("likes: 5"));
        assert!(line.contains("comments: 2"));
        assert!(line.contains("shares: 1"));
        assert!(line.contains("123_456"));
    }

    #[test]
    fn formats_post_with_missing_fields_using_defaults() {
        let post = json!({"id": "1"});
        let line = format_post_summary(&post);
        assert!(line.contains("(no text)"));
        assert!(line.contains("likes: 0"));
        assert!(line.contains("shares: 0"));
    }
}
