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

    async fn list_conversations(&self, limit: u32) -> Result<String> {
        let page_id = self.credentials.page_id()?;
        let access_token = self.credentials.access_token()?;

        let client = Client::new();
        let url = self.credentials.graph_url(&format!("{}/conversations", page_id));
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
        let url = self.credentials.graph_url(&format!("{}/messages", conversation_id));
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
    let updated = conversation["updated_time"].as_str().unwrap_or("unknown time");
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
    format!(
        "[{}] {} (psid: {}): {}",
        created, from_name, from_id, text
    )
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
}
