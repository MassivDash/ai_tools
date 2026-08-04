use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::agent::tools::social::facebook_common::FacebookCredentials;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

/// Facebook Send Message Tool implementation
/// Replies to a person who has messaged the Page via Messenger (requires
/// the `pages_messaging` permission). Uses messaging_type "RESPONSE", which
/// Meta only allows within the standard customer-service window after the
/// person's last message — this is a reply tool, not a way to message
/// people who haven't contacted the Page.
pub struct FacebookSendMessageTool {
    metadata: ToolMetadata,
    credentials: FacebookCredentials,
}

impl FacebookSendMessageTool {
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                id: "facebook_send_message".to_string(),
                name: "Facebook Send Message".to_string(),
                description: "Reply to a Messenger conversation on the user's Facebook Page"
                    .to_string(),
                category: ToolCategory::Social,
                tool_type: ToolType::FacebookMessageSend,
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

    async fn send_message(&self, recipient_id: &str, message: &str) -> Result<String> {
        let access_token = self.credentials.access_token()?;

        let client = Client::new();
        let url = self.credentials.graph_url("me/messages");

        let body = json!({
            "recipient": {"id": recipient_id},
            "messaging_type": "RESPONSE",
            "message": {"text": message}
        });

        let response = client
            .post(&url)
            .query(&[("access_token", access_token)])
            .json(&body)
            .send()
            .await
            .context("Failed to connect to Facebook Graph API")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Failed to send Facebook message: {}",
                error_text
            ));
        }

        let data: serde_json::Value = response.json().await?;
        let message_id = data["message_id"].as_str().unwrap_or("unknown");

        Ok(format!(
            "Successfully sent message to {} (message id: {})",
            recipient_id, message_id
        ))
    }
}

#[async_trait]
impl AgentTool for FacebookSendMessageTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "facebook_send_message",
            "description": "Send a Messenger reply to someone who has messaged the Facebook Page. Only works within Meta's standard reply window after their last message; get the recipient_id from facebook_read_messages.",
            "parameters": {
                "type": "object",
                "properties": {
                    "recipient_id": {
                        "type": "string",
                        "description": "The Page-scoped user ID (PSID) of the person to reply to, as seen in facebook_read_messages output."
                    },
                    "message": {
                        "type": "string",
                        "description": "The text content of the reply."
                    }
                },
                "required": ["recipient_id", "message"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse tool call arguments")?;

        let recipient_id = args
            .get("recipient_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: recipient_id"))?;
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: message"))?;

        println!("📘 Sending Facebook message...");
        let result = self.send_message(recipient_id, message).await?;
        println!("✅ {}", result);

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "facebook_send_message".to_string(),
            result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::core::types::FunctionCall;
    use crate::test_support::{MockHttpApi, MockResponse};

    /// Messenger sends go to /me/messages, addressed by token rather than page id.
    const MESSAGES_PATH: &str = "/v21.0/me/messages";

    fn tool_call(arguments: &str) -> ToolCall {
        ToolCall {
            id: "call_fb_send".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "facebook_send_message".to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    fn tool_for(api: &MockHttpApi) -> FacebookSendMessageTool {
        FacebookSendMessageTool::with_credentials(FacebookCredentials::for_test(api.base_url()))
    }

    #[test]
    fn metadata_and_function_definition_describe_the_send_tool() {
        let tool = FacebookSendMessageTool::new();
        assert_eq!(tool.metadata().id, "facebook_send_message");
        assert_eq!(tool.metadata().category, ToolCategory::Social);
        assert_eq!(tool.metadata().tool_type, ToolType::FacebookMessageSend);

        let def = tool.get_function_definition();
        assert_eq!(def["name"], "facebook_send_message");
        assert_eq!(
            def["parameters"]["required"],
            json!(["recipient_id", "message"])
        );
    }

    #[tokio::test]
    async fn a_reply_is_sent_as_a_response_type_message() {
        let api = MockHttpApi::serving(
            "POST",
            MESSAGES_PATH,
            MockResponse::json(json!({"recipient_id": "psid_9", "message_id": "mid.42"})),
        )
        .await;

        let result = tool_for(&api)
            .execute(&tool_call(
                r#"{"recipient_id": "psid_9", "message": "On its way!"}"#,
            ))
            .await
            .expect("The send should succeed");

        let request = api.only_request();
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, MESSAGES_PATH);
        // The token travels as a query parameter, the message as a JSON body.
        assert_eq!(
            request.query_param("access_token").as_deref(),
            Some("test-page-token")
        );
        assert_eq!(
            request.json(),
            json!({
                "recipient": {"id": "psid_9"},
                "messaging_type": "RESPONSE",
                "message": {"text": "On its way!"}
            })
        );

        assert_eq!(result.tool_name, "facebook_send_message");
        assert!(result.tool_call_id.is_none());
        assert_eq!(
            result.result,
            "Successfully sent message to psid_9 (message id: mid.42)"
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn a_response_without_a_message_id_degrades() {
        let api = MockHttpApi::serving("POST", MESSAGES_PATH, MockResponse::json(json!({}))).await;

        let result = tool_for(&api)
            .execute(&tool_call(r#"{"recipient_id": "psid_9", "message": "hi"}"#))
            .await
            .expect("The send should succeed");

        assert_eq!(
            result.result,
            "Successfully sent message to psid_9 (message id: unknown)"
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn an_outside_the_window_error_is_surfaced() {
        let api = MockHttpApi::serving(
            "POST",
            MESSAGES_PATH,
            MockResponse::error(
                400,
                r#"{"error":{"message":"This message is sent outside of allowed window","code":10}}"#,
            ),
        )
        .await;

        let error = tool_for(&api)
            .execute(&tool_call(r#"{"recipient_id": "psid_9", "message": "hi"}"#))
            .await
            .expect_err("A 400 must fail the call");

        let message = error.to_string();
        assert!(
            message.starts_with("Failed to send Facebook message:"),
            "{}",
            message
        );
        assert!(message.contains("outside of allowed window"), "{}", message);
        api.stop().await;
    }

    #[tokio::test]
    async fn bad_arguments_and_a_missing_token_fail_before_any_request() {
        let api = MockHttpApi::serving("POST", MESSAGES_PATH, MockResponse::json(json!({}))).await;
        let tool = tool_for(&api);

        assert_eq!(
            tool.execute(&tool_call("~"))
                .await
                .expect_err("Unparseable arguments must fail")
                .to_string(),
            "Failed to parse tool call arguments"
        );
        assert_eq!(
            tool.execute(&tool_call(r#"{"message": "hi"}"#))
                .await
                .expect_err("A missing recipient must fail")
                .to_string(),
            "Missing required parameter: recipient_id"
        );
        assert_eq!(
            tool.execute(&tool_call(r#"{"recipient_id": "psid_9"}"#))
                .await
                .expect_err("A missing message must fail")
                .to_string(),
            "Missing required parameter: message"
        );

        let tokenless = FacebookSendMessageTool::with_credentials(
            FacebookCredentials::for_test(api.base_url()).without_access_token(),
        );
        assert_eq!(
            tokenless
                .execute(&tool_call(r#"{"recipient_id": "psid_9", "message": "hi"}"#))
                .await
                .expect_err("Without a token the call must fail")
                .to_string(),
            "FACEBOOK_PAGE_ACCESS_TOKEN environment variable not set"
        );

        assert_eq!(api.call_count(), 0, "Nothing should have reached the API");
        api.stop().await;
    }
}
