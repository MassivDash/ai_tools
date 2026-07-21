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
