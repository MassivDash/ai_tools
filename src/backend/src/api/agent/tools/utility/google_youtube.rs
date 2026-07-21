use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::agent::tools::utility::google_oauth::GoogleOAuthProvider;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct GoogleYouTubeReadTool {
    metadata: ToolMetadata,
    oauth_provider: Arc<GoogleOAuthProvider>,
}

impl GoogleYouTubeReadTool {
    pub fn new(oauth_provider: Arc<GoogleOAuthProvider>) -> Self {
        Self {
            metadata: ToolMetadata {
                id: "google_youtube_read".to_string(),
                name: "Read YouTube Videos".to_string(),
                description: "Read data from YouTube (e.g. liked videos, search).".to_string(),
                category: ToolCategory::Google,
                tool_type: ToolType::GoogleYouTubeRead,
            },
            oauth_provider,
        }
    }
}

#[async_trait]
impl AgentTool for GoogleYouTubeReadTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "google_youtube_read",
            "description": "Read data from YouTube (liked videos or search).",
            "parameters": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "The action to perform: 'liked' for liked videos, 'search' to search for videos.",
                        "enum": ["liked", "search"]
                    },
                    "query": {
                        "type": "string",
                        "description": "Search query. Required if action is 'search'."
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "The maximum number of videos to return. Defaults to 5."
                    }
                },
                "required": ["action"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).unwrap_or_else(|_| json!({}));

        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("liked");
        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(5);

        println!(
            "\x1b[36m▶️ Reading YouTube with action: '{}'\x1b[0m",
            action
        );

        let access_token = self.oauth_provider.get_access_token().await?;
        let client = self.oauth_provider.http_client.clone();

        let res = if action == "search" {
            client
                .get("https://www.googleapis.com/youtube/v3/search")
                .bearer_auth(&access_token)
                .query(&[
                    ("part", "snippet"),
                    ("q", query),
                    ("type", "video"),
                    ("maxResults", &max_results.to_string()),
                ])
                .send()
                .await
                .context("Failed to search YouTube videos")?
        } else {
            client
                .get("https://www.googleapis.com/youtube/v3/videos")
                .bearer_auth(&access_token)
                .query(&[
                    ("part", "snippet"),
                    ("myRating", "like"),
                    ("maxResults", &max_results.to_string()),
                ])
                .send()
                .await
                .context("Failed to get liked YouTube videos")?
        };

        if !res.status().is_success() {
            return Err(anyhow::anyhow!(
                "YouTube API failed: {}",
                res.text().await.unwrap_or_default()
            ));
        }

        let doc: serde_json::Value = res.json().await.unwrap_or_default();
        let items = doc.get("items").and_then(|v| v.as_array());

        let mut results = Vec::new();
        if let Some(videos) = items {
            for video in videos {
                // The ID structure differs between /videos and /search endpoints
                let id = if action == "search" {
                    video
                        .get("id")
                        .and_then(|v| v.get("videoId"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                } else {
                    video.get("id").and_then(|v| v.as_str()).unwrap_or("")
                };

                let snippet = video.get("snippet");
                let title = snippet
                    .and_then(|s| s.get("title"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let channel = snippet
                    .and_then(|s| s.get("channelTitle"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                results.push(format!(
                    "Video ID: {}\nTitle: {}\nChannel: {}\nUrl: https://www.youtube.com/watch?v={}\n---",
                    id, title, channel, id
                ));
            }
        }

        if results.is_empty() {
            return Ok(ToolCallResult {
                tool_call_id: None,
                tool_name: "google_youtube_read".to_string(),
                result: "No videos found.".to_string(),
            });
        }

        println!("\x1b[32m✅ Successfully read YouTube data\x1b[0m");

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "google_youtube_read".to_string(),
            result: format!("Found {} videos:\n\n{}", results.len(), results.join("\n")),
        })
    }

    fn is_available(&self) -> bool {
        GoogleOAuthProvider::is_configured()
    }
}
