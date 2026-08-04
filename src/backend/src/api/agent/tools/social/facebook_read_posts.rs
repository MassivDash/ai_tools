use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::agent::tools::social::facebook_common::FacebookCredentials;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

const FIELDS: &str =
    "id,message,created_time,permalink_url,likes.summary(true),comments.summary(true),shares";

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

    /// A tool using `credentials` instead of the process environment, so tests
    /// can point it at a loopback mock Graph API.
    #[cfg(test)]
    pub(crate) fn with_credentials(credentials: FacebookCredentials) -> Self {
        Self {
            credentials,
            ..Self::new()
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

        Ok(posts
            .iter()
            .map(format_post_summary)
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

/// Renders a single Graph API post object as a one-line summary.
fn format_post_summary(post: &serde_json::Value) -> String {
    let id = post["id"].as_str().unwrap_or("unknown");
    let message = post["message"].as_str().unwrap_or("(no text)");
    let created = post["created_time"].as_str().unwrap_or("unknown time");
    let likes = post["likes"]["summary"]["total_count"]
        .as_u64()
        .unwrap_or(0);
    let comments = post["comments"]["summary"]["total_count"]
        .as_u64()
        .unwrap_or(0);
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

    use crate::api::agent::core::types::FunctionCall;
    use crate::test_support::{MockHttpApi, MockResponse};

    /// Where `FacebookCredentials::for_test` (page_1) reads posts from.
    const POSTS_PATH: &str = "/v21.0/page_1/posts";

    fn tool_call(arguments: &str) -> ToolCall {
        ToolCall {
            id: "call_fb_posts".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "facebook_read_posts".to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    fn tool_for(api: &MockHttpApi) -> FacebookReadPostsTool {
        FacebookReadPostsTool::with_credentials(FacebookCredentials::for_test(api.base_url()))
    }

    #[test]
    fn metadata_and_function_definition_describe_the_read_posts_tool() {
        let tool = FacebookReadPostsTool::new();
        assert_eq!(tool.metadata().id, "facebook_read_posts");
        assert_eq!(tool.metadata().category, ToolCategory::Social);
        assert_eq!(tool.metadata().tool_type, ToolType::FacebookPostsRead);

        let def = tool.get_function_definition();
        assert_eq!(def["name"], "facebook_read_posts");
        assert_eq!(def["parameters"]["required"], json!([]));
    }

    #[tokio::test]
    async fn posts_are_requested_with_engagement_fields_and_rendered_one_per_line() {
        let api = MockHttpApi::serving(
            "GET",
            POSTS_PATH,
            MockResponse::json(json!({"data": [
                {
                    "id": "page_1_1",
                    "message": "First",
                    "created_time": "2026-08-01T10:00:00+0000",
                    "likes": {"summary": {"total_count": 3}},
                    "comments": {"summary": {"total_count": 1}},
                    "shares": {"count": 2}
                },
                {"id": "page_1_2"}
            ]})),
        )
        .await;

        let result = tool_for(&api)
            .execute(&tool_call("{}"))
            .await
            .expect("Reading posts should succeed");

        let request = api.only_request();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, POSTS_PATH);
        assert_eq!(request.query_param("fields").as_deref(), Some(FIELDS));
        // The default limit is 10 when the caller does not ask for one.
        assert_eq!(request.query_param("limit").as_deref(), Some("10"));
        assert_eq!(
            request.query_param("access_token").as_deref(),
            Some("test-page-token")
        );

        assert_eq!(result.tool_name, "facebook_read_posts");
        assert!(result.tool_call_id.is_none());
        let lines: Vec<&str> = result.result.lines().collect();
        assert_eq!(lines.len(), 2, "{}", result.result);
        assert!(lines[0].contains("First"));
        assert!(lines[0].contains("likes: 3 comments: 1 shares: 2"));
        assert!(lines[1].contains("(no text)"));
        api.stop().await;
    }

    #[tokio::test]
    async fn the_limit_is_clamped_to_the_documented_maximum() {
        let api =
            MockHttpApi::serving("GET", POSTS_PATH, MockResponse::json(json!({"data": []}))).await;
        let tool = tool_for(&api);

        tool.execute(&tool_call(r#"{"limit": 500}"#))
            .await
            .expect("An over-large limit is clamped, not rejected");
        tool.execute(&tool_call(r#"{"limit": 0}"#))
            .await
            .expect("A zero limit is clamped, not rejected");

        let limits: Vec<Option<String>> = api
            .requests()
            .iter()
            .map(|request| request.query_param("limit"))
            .collect();
        assert_eq!(
            limits,
            vec![Some("25".to_string()), Some("1".to_string())],
            "limit must be clamped into 1..=25"
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn an_empty_page_says_so() {
        let api =
            MockHttpApi::serving("GET", POSTS_PATH, MockResponse::json(json!({"data": []}))).await;

        let result = tool_for(&api)
            .execute(&tool_call("{}"))
            .await
            .expect("An empty data array is not an error");

        assert_eq!(result.result, "No posts found on the Page.");
        api.stop().await;
    }

    #[tokio::test]
    async fn a_graph_api_error_body_is_surfaced() {
        let api = MockHttpApi::serving(
            "GET",
            POSTS_PATH,
            MockResponse::error(
                403,
                r#"{"error":{"message":"(#210) requires pages_read_engagement"}}"#,
            ),
        )
        .await;

        let error = tool_for(&api)
            .execute(&tool_call("{}"))
            .await
            .expect_err("A 403 must fail the call");

        let message = error.to_string();
        assert!(
            message.starts_with("Failed to read Facebook posts:"),
            "{}",
            message
        );
        assert!(message.contains("pages_read_engagement"), "{}", message);
        api.stop().await;
    }

    #[tokio::test]
    async fn missing_credentials_fail_before_any_request() {
        let api =
            MockHttpApi::serving("GET", POSTS_PATH, MockResponse::json(json!({"data": []}))).await;
        let tokenless = FacebookReadPostsTool::with_credentials(
            FacebookCredentials::for_test(api.base_url()).without_access_token(),
        );

        assert_eq!(
            tokenless
                .execute(&tool_call("{}"))
                .await
                .expect_err("Without a token the call must fail")
                .to_string(),
            "FACEBOOK_PAGE_ACCESS_TOKEN environment variable not set"
        );
        assert_eq!(
            tool_for(&api)
                .execute(&tool_call("@"))
                .await
                .expect_err("Unparseable arguments must fail")
                .to_string(),
            "Failed to parse tool call arguments"
        );

        assert_eq!(api.call_count(), 0, "Nothing should have reached the API");
        api.stop().await;
    }
}
