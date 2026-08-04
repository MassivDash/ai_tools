use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::agent::tools::social::facebook_common::FacebookCredentials;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

const CONVERSATION_FIELDS: &str = "participants,updated_time,snippet";
const MESSAGE_FIELDS: &str = "message,from,created_time";

/// Facebook Read Messages Tool implementation
/// Lists Page inbox conversations, or the messages within one conversation
/// (requires the `pages_messaging` permission).
pub struct FacebookReadMessagesTool {
    metadata: ToolMetadata,
    credentials: FacebookCredentials,
}

impl FacebookReadMessagesTool {
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                id: "facebook_read_messages".to_string(),
                name: "Facebook Read Messages".to_string(),
                description: "Read the Facebook Page's Messenger inbox".to_string(),
                category: ToolCategory::Social,
                tool_type: ToolType::FacebookMessagesRead,
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

    async fn list_conversations(&self, limit: u32) -> Result<String> {
        let page_id = self.credentials.page_id()?;
        let access_token = self.credentials.access_token()?;

        let client = Client::new();
        let url = self
            .credentials
            .graph_url(&format!("{}/conversations", page_id));
        let limit_str = limit.to_string();

        let response = client
            .get(&url)
            .query(&[
                ("fields", CONVERSATION_FIELDS),
                ("limit", &limit_str),
                ("access_token", access_token),
            ])
            .send()
            .await
            .context("Failed to connect to Facebook Graph API")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Failed to read Facebook conversations: {}",
                error_text
            ));
        }

        let data: serde_json::Value = response.json().await?;
        let conversations = data["data"].as_array().cloned().unwrap_or_default();

        if conversations.is_empty() {
            return Ok("No conversations found in the Page inbox.".to_string());
        }

        Ok(conversations
            .iter()
            .map(format_conversation_summary)
            .collect::<Vec<_>>()
            .join("\n"))
    }

    async fn list_messages(&self, conversation_id: &str, limit: u32) -> Result<String> {
        let access_token = self.credentials.access_token()?;

        let client = Client::new();
        let url = self
            .credentials
            .graph_url(&format!("{}/messages", conversation_id));
        let limit_str = limit.to_string();

        let response = client
            .get(&url)
            .query(&[
                ("fields", MESSAGE_FIELDS),
                ("limit", &limit_str),
                ("access_token", access_token),
            ])
            .send()
            .await
            .context("Failed to connect to Facebook Graph API")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Failed to read Facebook messages: {}",
                error_text
            ));
        }

        let data: serde_json::Value = response.json().await?;
        let messages = data["data"].as_array().cloned().unwrap_or_default();

        if messages.is_empty() {
            return Ok("No messages found in this conversation.".to_string());
        }

        Ok(messages
            .iter()
            .map(format_message_line)
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

/// Renders a single Graph API conversation object as a one-line summary,
/// including each participant's PSID needed to reply via
/// facebook_send_message.
fn format_conversation_summary(conversation: &serde_json::Value) -> String {
    let id = conversation["id"].as_str().unwrap_or("unknown");
    let updated = conversation["updated_time"]
        .as_str()
        .unwrap_or("unknown time");
    let snippet = conversation["snippet"].as_str().unwrap_or("(no preview)");
    let participants = conversation["participants"]["data"]
        .as_array()
        .map(|people| {
            people
                .iter()
                .map(format_participant)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "unknown".to_string());
    format!(
        "[{}] {}: {} | conversation_id: {}",
        updated, participants, snippet, id
    )
}

/// Renders a single participant as "Name (psid: 123)" so the ID needed by
/// facebook_send_message's recipient_id is visible in the tool's output.
fn format_participant(participant: &serde_json::Value) -> String {
    let name = participant["name"].as_str().unwrap_or("Unknown");
    let id = participant["id"].as_str().unwrap_or("unknown");
    format!("{} (psid: {})", name, id)
}

/// Renders a single Graph API message object as a one-line summary,
/// including the sender's PSID needed to reply via facebook_send_message.
fn format_message_line(message: &serde_json::Value) -> String {
    let from_name = message["from"]["name"].as_str().unwrap_or("Unknown sender");
    let from_id = message["from"]["id"].as_str().unwrap_or("unknown");
    let text = message["message"].as_str().unwrap_or("(no text)");
    let created = message["created_time"].as_str().unwrap_or("unknown time");
    format!("[{}] {} (psid: {}): {}", created, from_name, from_id, text)
}

#[async_trait]
impl AgentTool for FacebookReadMessagesTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "facebook_read_messages",
            "description": "Read the Facebook Page's Messenger inbox. Omit conversation_id to list recent conversations; pass one to read the messages inside it.",
            "parameters": {
                "type": "object",
                "properties": {
                    "conversation_id": {
                        "type": "string",
                        "description": "Optional conversation ID (from a prior call) to read messages within a specific conversation."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of conversations or messages to return (default 10, max 25)."
                    }
                },
                "required": []
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse tool call arguments")?;

        let conversation_id = args.get("conversation_id").and_then(|v| v.as_str());
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n.clamp(1, 25) as u32)
            .unwrap_or(10);

        println!("📘 Reading Facebook messages...");
        let result = match conversation_id {
            Some(id) => self.list_messages(id, limit).await?,
            None => self.list_conversations(limit).await?,
        };

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "facebook_read_messages".to_string(),
            result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_conversation_with_participant_psids() {
        // Regression: facebook_send_message's recipient_id must be
        // recoverable from this tool's own output.
        let conversation = json!({
            "id": "t_123",
            "updated_time": "2026-07-20T10:00:00+0000",
            "snippet": "Hey there",
            "participants": {"data": [{"name": "Jane Doe", "id": "psid_789"}, {"name": "Page Name", "id": "page_1"}]}
        });
        let line = format_conversation_summary(&conversation);
        assert!(line.contains("Jane Doe"));
        assert!(line.contains("psid_789"));
        assert!(line.contains("Hey there"));
        assert!(line.contains("t_123"));
    }

    #[test]
    fn formats_message_with_sender_psid() {
        // Regression: the sender's PSID must be present so it can be fed
        // straight into facebook_send_message's recipient_id parameter.
        let message = json!({
            "from": {"name": "Jane Doe", "id": "psid_789"},
            "message": "Hello!",
            "created_time": "2026-07-20T10:00:00+0000"
        });
        let line = format_message_line(&message);
        assert!(line.contains("Jane Doe"));
        assert!(line.contains("psid_789"));
        assert!(line.contains("Hello!"));
    }

    use crate::api::agent::core::types::FunctionCall;
    use crate::test_support::{MockHttpApi, MockResponse};

    const CONVERSATIONS_PATH: &str = "/v21.0/page_1/conversations";

    fn tool_call(arguments: &str) -> ToolCall {
        ToolCall {
            id: "call_fb_messages".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "facebook_read_messages".to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    fn tool_for(api: &MockHttpApi) -> FacebookReadMessagesTool {
        FacebookReadMessagesTool::with_credentials(FacebookCredentials::for_test(api.base_url()))
    }

    #[test]
    fn metadata_and_function_definition_describe_the_read_messages_tool() {
        let tool = FacebookReadMessagesTool::new();
        assert_eq!(tool.metadata().id, "facebook_read_messages");
        assert_eq!(tool.metadata().category, ToolCategory::Social);
        assert_eq!(tool.metadata().tool_type, ToolType::FacebookMessagesRead);

        let def = tool.get_function_definition();
        assert_eq!(def["name"], "facebook_read_messages");
        assert_eq!(def["parameters"]["required"], json!([]));
    }

    #[tokio::test]
    async fn without_a_conversation_id_the_page_inbox_is_listed() {
        let api = MockHttpApi::serving(
            "GET",
            CONVERSATIONS_PATH,
            MockResponse::json(json!({"data": [{
                "id": "t_123",
                "updated_time": "2026-08-01T10:00:00+0000",
                "snippet": "Hey there",
                "participants": {"data": [
                    {"name": "Jane Doe", "id": "psid_789"},
                    {"name": "My Page", "id": "page_1"}
                ]}
            }]})),
        )
        .await;

        let result = tool_for(&api)
            .execute(&tool_call("{}"))
            .await
            .expect("Listing conversations should succeed");

        let request = api.only_request();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, CONVERSATIONS_PATH);
        assert_eq!(
            request.query_param("fields").as_deref(),
            Some(CONVERSATION_FIELDS)
        );
        assert_eq!(request.query_param("limit").as_deref(), Some("10"));
        assert_eq!(
            request.query_param("access_token").as_deref(),
            Some("test-page-token")
        );

        assert_eq!(result.tool_name, "facebook_read_messages");
        assert!(result.tool_call_id.is_none());
        assert!(result.result.contains("Jane Doe (psid: psid_789)"));
        assert!(result.result.contains("Hey there"));
        assert!(result.result.contains("conversation_id: t_123"));
        api.stop().await;
    }

    #[tokio::test]
    async fn with_a_conversation_id_that_conversations_messages_are_listed() {
        let api = MockHttpApi::serving(
            "GET",
            "/v21.0/t_123/messages",
            MockResponse::json(json!({"data": [{
                "from": {"name": "Jane Doe", "id": "psid_789"},
                "message": "Hello!",
                "created_time": "2026-08-01T10:05:00+0000"
            }]})),
        )
        .await;

        let result = tool_for(&api)
            .execute(&tool_call(r#"{"conversation_id": "t_123", "limit": 3}"#))
            .await
            .expect("Listing messages should succeed");

        let request = api.only_request();
        assert_eq!(request.path, "/v21.0/t_123/messages");
        assert_eq!(
            request.query_param("fields").as_deref(),
            Some(MESSAGE_FIELDS)
        );
        assert_eq!(request.query_param("limit").as_deref(), Some("3"));

        assert!(result.result.contains("Jane Doe (psid: psid_789): Hello!"));
        api.stop().await;
    }

    #[tokio::test]
    async fn empty_results_are_reported_per_endpoint() {
        let api = MockHttpApi::start().await;
        api.on(
            "GET",
            CONVERSATIONS_PATH,
            MockResponse::json(json!({"data": []})),
        );
        api.on(
            "GET",
            "/v21.0/t_1/messages",
            MockResponse::json(json!({"other": "shape"})),
        );
        let tool = tool_for(&api);

        assert_eq!(
            tool.execute(&tool_call("{}"))
                .await
                .expect("An empty inbox is not an error")
                .result,
            "No conversations found in the Page inbox."
        );
        // A body with no "data" key at all degrades the same way.
        assert_eq!(
            tool.execute(&tool_call(r#"{"conversation_id": "t_1"}"#))
                .await
                .expect("A body without data is not an error")
                .result,
            "No messages found in this conversation."
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn graph_api_errors_are_surfaced_for_both_endpoints() {
        let api = MockHttpApi::start().await;
        api.on(
            "GET",
            CONVERSATIONS_PATH,
            MockResponse::error(
                403,
                r#"{"error":{"message":"(#3) requires pages_messaging"}}"#,
            ),
        );
        api.on(
            "GET",
            "/v21.0/t_1/messages",
            MockResponse::error(400, r#"{"error":{"message":"Invalid conversation id"}}"#),
        );
        let tool = tool_for(&api);

        let inbox_error = tool
            .execute(&tool_call("{}"))
            .await
            .expect_err("A 403 must fail the call")
            .to_string();
        assert!(
            inbox_error.starts_with("Failed to read Facebook conversations:"),
            "{}",
            inbox_error
        );
        assert!(inbox_error.contains("pages_messaging"), "{}", inbox_error);

        let message_error = tool
            .execute(&tool_call(r#"{"conversation_id": "t_1"}"#))
            .await
            .expect_err("A 400 must fail the call")
            .to_string();
        assert!(
            message_error.starts_with("Failed to read Facebook messages:"),
            "{}",
            message_error
        );
        assert!(
            message_error.contains("Invalid conversation id"),
            "{}",
            message_error
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn bad_arguments_and_a_missing_token_fail_before_any_request() {
        let api = MockHttpApi::serving(
            "GET",
            CONVERSATIONS_PATH,
            MockResponse::json(json!({"data": []})),
        )
        .await;

        assert_eq!(
            tool_for(&api)
                .execute(&tool_call("^"))
                .await
                .expect_err("Unparseable arguments must fail")
                .to_string(),
            "Failed to parse tool call arguments"
        );

        let tokenless = FacebookReadMessagesTool::with_credentials(
            FacebookCredentials::for_test(api.base_url()).without_access_token(),
        );
        assert_eq!(
            tokenless
                .execute(&tool_call("{}"))
                .await
                .expect_err("Without a token the inbox listing must fail")
                .to_string(),
            "FACEBOOK_PAGE_ACCESS_TOKEN environment variable not set"
        );
        assert_eq!(
            tokenless
                .execute(&tool_call(r#"{"conversation_id": "t_1"}"#))
                .await
                .expect_err("Without a token the message listing must fail")
                .to_string(),
            "FACEBOOK_PAGE_ACCESS_TOKEN environment variable not set"
        );

        assert_eq!(api.call_count(), 0, "Nothing should have reached the API");
        api.stop().await;
    }
}
