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

    /// A tool using `credentials` instead of the process environment, so tests
    /// can point it at a loopback mock Graph API.
    #[cfg(test)]
    pub(crate) fn with_credentials(credentials: FacebookCredentials) -> Self {
        Self {
            credentials,
            ..Self::new()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::core::types::FunctionCall;
    use crate::test_support::{MockHttpApi, MockResponse};

    /// Where `FacebookCredentials::for_test` (page_1) posts, under the default
    /// Graph API version.
    const FEED_PATH: &str = "/v21.0/page_1/feed";

    fn tool_call(arguments: &str) -> ToolCall {
        ToolCall {
            id: "call_fb_post".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "facebook_post".to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    fn tool_for(api: &MockHttpApi) -> FacebookPostTool {
        FacebookPostTool::with_credentials(FacebookCredentials::for_test(api.base_url()))
    }

    #[test]
    fn metadata_and_function_definition_describe_the_post_tool() {
        let tool = FacebookPostTool::new();
        assert_eq!(tool.metadata().id, "facebook_post");
        assert_eq!(tool.metadata().category, ToolCategory::Social);
        assert_eq!(tool.metadata().tool_type, ToolType::FacebookPost);

        let def = tool.get_function_definition();
        assert_eq!(def["name"], "facebook_post");
        assert_eq!(def["parameters"]["required"], json!(["message"]));
    }

    #[tokio::test]
    async fn a_post_is_form_encoded_to_the_page_feed() {
        let api = MockHttpApi::serving(
            "POST",
            FEED_PATH,
            MockResponse::json(json!({"id": "page_1_999"})),
        )
        .await;

        let result = tool_for(&api)
            .execute(&tool_call(r#"{"message": "Hello, world & co"}"#))
            .await
            .expect("The post should succeed");

        let request = api.only_request();
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, FEED_PATH);
        assert_eq!(
            request.header("content-type"),
            Some("application/x-www-form-urlencoded")
        );
        assert_eq!(
            request.form_param("message").as_deref(),
            Some("Hello, world & co")
        );
        assert_eq!(
            request.form_param("access_token").as_deref(),
            Some("test-page-token")
        );
        assert!(
            request.form_param("link").is_none(),
            "No link should be sent when none was given: {}",
            request.body_text()
        );

        assert_eq!(result.tool_name, "facebook_post");
        assert!(result.tool_call_id.is_none());
        assert_eq!(
            result.result,
            "Successfully posted to Facebook Page (post id: page_1_999)"
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn an_optional_link_is_attached_and_a_missing_id_degrades() {
        let api = MockHttpApi::serving("POST", FEED_PATH, MockResponse::json(json!({}))).await;

        let result = tool_for(&api)
            .execute(&tool_call(
                r#"{"message": "look", "link": "https://example.com/a?b=c"}"#,
            ))
            .await
            .expect("The post should succeed");

        assert_eq!(
            api.only_request().form_param("link").as_deref(),
            Some("https://example.com/a?b=c")
        );
        assert_eq!(
            result.result,
            "Successfully posted to Facebook Page (post id: unknown)"
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn a_graph_api_error_body_is_surfaced() {
        let api = MockHttpApi::serving(
            "POST",
            FEED_PATH,
            MockResponse::error(
                400,
                r#"{"error":{"message":"(#200) Permissions error","type":"OAuthException"}}"#,
            ),
        )
        .await;

        let error = tool_for(&api)
            .execute(&tool_call(r#"{"message": "hi"}"#))
            .await
            .expect_err("A 400 must fail the call");

        let message = error.to_string();
        assert!(
            message.starts_with("Failed to post to Facebook:"),
            "{}",
            message
        );
        assert!(message.contains("(#200) Permissions error"), "{}", message);
        api.stop().await;
    }

    #[tokio::test]
    async fn a_missing_token_fails_before_any_request() {
        let api = MockHttpApi::serving("POST", FEED_PATH, MockResponse::json(json!({}))).await;
        let tool = FacebookPostTool::with_credentials(
            FacebookCredentials::for_test(api.base_url()).without_access_token(),
        );

        let error = tool
            .execute(&tool_call(r#"{"message": "hi"}"#))
            .await
            .expect_err("Without a token the call must fail");

        assert_eq!(
            error.to_string(),
            "FACEBOOK_PAGE_ACCESS_TOKEN environment variable not set"
        );
        assert_eq!(api.call_count(), 0, "Nothing should have reached the API");
        api.stop().await;
    }

    #[tokio::test]
    async fn bad_arguments_fail_before_any_request() {
        let api = MockHttpApi::serving("POST", FEED_PATH, MockResponse::json(json!({}))).await;
        let tool = tool_for(&api);

        assert_eq!(
            tool.execute(&tool_call("!"))
                .await
                .expect_err("Unparseable arguments must fail")
                .to_string(),
            "Failed to parse tool call arguments"
        );
        assert_eq!(
            tool.execute(&tool_call(r#"{"text": "wrong field"}"#))
                .await
                .expect_err("A missing message must fail")
                .to_string(),
            "Missing required parameter: message"
        );

        assert_eq!(api.call_count(), 0);
        api.stop().await;
    }
}
