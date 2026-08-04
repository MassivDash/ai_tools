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

    /// A tool using `credentials` instead of the process environment, so tests
    /// can point it at a loopback mock Graph API.
    #[cfg(test)]
    pub(crate) fn with_credentials(credentials: FacebookCredentials) -> Self {
        Self {
            credentials,
            ..Self::new()
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

    use crate::api::agent::core::types::FunctionCall;
    use crate::test_support::{MockHttpApi, MockResponse};

    fn tool_call(arguments: &str) -> ToolCall {
        ToolCall {
            id: "call_fb_comments".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "facebook_read_comments".to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    fn tool_for(api: &MockHttpApi) -> FacebookReadCommentsTool {
        FacebookReadCommentsTool::with_credentials(FacebookCredentials::for_test(api.base_url()))
    }

    #[test]
    fn metadata_and_function_definition_describe_the_read_comments_tool() {
        let tool = FacebookReadCommentsTool::new();
        assert_eq!(tool.metadata().id, "facebook_read_comments");
        assert_eq!(tool.metadata().category, ToolCategory::Social);
        assert_eq!(tool.metadata().tool_type, ToolType::FacebookCommentsRead);

        let def = tool.get_function_definition();
        assert_eq!(def["name"], "facebook_read_comments");
        assert_eq!(def["parameters"]["required"], json!(["post_id"]));
    }

    #[tokio::test]
    async fn comments_are_read_from_the_post_the_caller_named() {
        // The post id is interpolated straight into the Graph path.
        let api = MockHttpApi::serving(
            "GET",
            "/v21.0/page_1_77/comments",
            MockResponse::json(json!({"data": [
                {
                    "id": "c1",
                    "from": {"name": "Jane Doe"},
                    "message": "Great post!",
                    "created_time": "2026-08-01T10:00:00+0000"
                },
                {"id": "c2"}
            ]})),
        )
        .await;

        let result = tool_for(&api)
            .execute(&tool_call(r#"{"post_id": "page_1_77", "limit": 5}"#))
            .await
            .expect("Reading comments should succeed");

        let request = api.only_request();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/v21.0/page_1_77/comments");
        assert_eq!(request.query_param("fields").as_deref(), Some(FIELDS));
        assert_eq!(request.query_param("limit").as_deref(), Some("5"));
        assert_eq!(
            request.query_param("access_token").as_deref(),
            Some("test-page-token")
        );

        assert_eq!(result.tool_name, "facebook_read_comments");
        assert!(result.tool_call_id.is_none());
        let lines: Vec<&str> = result.result.lines().collect();
        assert_eq!(lines.len(), 2, "{}", result.result);
        assert!(lines[0].contains("Jane Doe: Great post!"));
        assert!(lines[1].contains("Unknown user: (no text)"));
        api.stop().await;
    }

    #[tokio::test]
    async fn an_unrelated_post_id_is_not_silently_rewritten() {
        // Only the exact /{post_id}/comments path is mocked, so a tool that
        // addressed anything else would 404 here instead of passing.
        let api = MockHttpApi::serving(
            "GET",
            "/v21.0/999_888/comments",
            MockResponse::json(json!({"data": []})),
        )
        .await;

        let result = tool_for(&api)
            .execute(&tool_call(r#"{"post_id": "999_888"}"#))
            .await
            .expect("Reading comments should succeed");

        assert_eq!(result.result, "No comments found on this post.");
        assert_eq!(
            api.only_request().query_param("limit").as_deref(),
            Some("10"),
            "The documented default limit is 10"
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn a_graph_api_error_body_is_surfaced() {
        let api = MockHttpApi::serving(
            "GET",
            "/v21.0/p1/comments",
            MockResponse::error(400, r#"{"error":{"message":"Unsupported get request"}}"#),
        )
        .await;

        let error = tool_for(&api)
            .execute(&tool_call(r#"{"post_id": "p1"}"#))
            .await
            .expect_err("A 400 must fail the call");

        let message = error.to_string();
        assert!(
            message.starts_with("Failed to read Facebook comments:"),
            "{}",
            message
        );
        assert!(message.contains("Unsupported get request"), "{}", message);
        api.stop().await;
    }

    #[tokio::test]
    async fn bad_arguments_and_a_missing_token_fail_before_any_request() {
        let api = MockHttpApi::serving(
            "GET",
            "/v21.0/p1/comments",
            MockResponse::json(json!({"data": []})),
        )
        .await;
        let tool = tool_for(&api);

        assert_eq!(
            tool.execute(&tool_call("%"))
                .await
                .expect_err("Unparseable arguments must fail")
                .to_string(),
            "Failed to parse tool call arguments"
        );
        assert_eq!(
            tool.execute(&tool_call("{}"))
                .await
                .expect_err("A missing post_id must fail")
                .to_string(),
            "Missing required parameter: post_id"
        );

        let tokenless = FacebookReadCommentsTool::with_credentials(
            FacebookCredentials::for_test(api.base_url()).without_access_token(),
        );
        assert_eq!(
            tokenless
                .execute(&tool_call(r#"{"post_id": "p1"}"#))
                .await
                .expect_err("Without a token the call must fail")
                .to_string(),
            "FACEBOOK_PAGE_ACCESS_TOKEN environment variable not set"
        );

        assert_eq!(api.call_count(), 0, "Nothing should have reached the API");
        api.stop().await;
    }
}
