use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::agent::tools::utility::google_oauth::GoogleOAuthProvider;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct GoogleGmailReadTool {
    metadata: ToolMetadata,
    oauth_provider: Arc<GoogleOAuthProvider>,
}

impl GoogleGmailReadTool {
    pub fn new(oauth_provider: Arc<GoogleOAuthProvider>) -> Self {
        Self {
            metadata: ToolMetadata {
                id: "read_gmail_oauth".to_string(),
                name: "Read Emails (Gmail)".to_string(),
                description: "Read recent emails from the user's Gmail account.".to_string(),
                category: ToolCategory::Utility,
                tool_type: ToolType::GoogleGmailRead,
            },
            oauth_provider,
        }
    }
}

#[async_trait]
impl AgentTool for GoogleGmailReadTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "read_gmail_oauth",
            "description": "Read recent emails from the user's Gmail account.",
            "parameters": {
                "type": "object",
                "properties": {
                    "max_results": {
                        "type": "integer",
                        "description": "The maximum number of emails to return. Defaults to 5."
                    },
                    "query": {
                        "type": "string",
                        "description": "Optional search query to filter emails (e.g. 'is:unread', 'from:boss@example.com')."
                    }
                }
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).unwrap_or_else(|_| json!({}));

        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(5);
        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");

        println!(
            "\x1b[36m📧 Reading {} latest Gmail messages with query '{}'\x1b[0m",
            max_results, query
        );

        let access_token = self.oauth_provider.get_access_token().await?;
        let client = self.oauth_provider.http_client.clone();

        // 1. Fetch message list
        let mut list_req = client
            .get("https://gmail.googleapis.com/gmail/v1/users/me/messages")
            .bearer_auth(&access_token)
            .query(&[("maxResults", &max_results.to_string())]);

        if !query.is_empty() {
            list_req = list_req.query(&[("q", query)]);
        }

        let list_res = list_req
            .send()
            .await
            .context("Failed to list Gmail messages")?;

        if !list_res.status().is_success() {
            let status = list_res.status();
            let text = list_res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Gmail API list failed with {}: {}",
                status,
                text
            ));
        }

        let list_data: serde_json::Value = list_res.json().await.unwrap_or_default();
        let messages = list_data.get("messages").and_then(|v| v.as_array());

        let messages_array = match messages {
            Some(arr) if !arr.is_empty() => arr,
            _ => {
                return Ok(ToolCallResult {
                    tool_name: "read_gmail_oauth".to_string(),
                    result: "No emails found matching the criteria.".to_string(),
                });
            }
        };

        // 2. Fetch individual message details concurrently
        let mut fetch_handles = Vec::new();

        for msg in messages_array {
            if let Some(id) = msg.get("id").and_then(|v| v.as_str()) {
                let id_string = id.to_string();
                let access_token_clone = access_token.clone();
                let client_clone = client.clone();

                let handle = tokio::spawn(async move {
                    let msg_res = client_clone
                        .get(format!("https://gmail.googleapis.com/gmail/v1/users/me/messages/{}?format=metadata&metadataHeaders=Subject&metadataHeaders=From&metadataHeaders=Date", id_string))
                        .bearer_auth(&access_token_clone)
                        .send()
                        .await;

                    if let Ok(res) = msg_res {
                        if res.status().is_success() {
                            let msg_data: serde_json::Value = res.json().await.unwrap_or_default();

                            let snippet = msg_data.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
                            let payload = msg_data.get("payload");
                            let headers = payload.and_then(|p| p.get("headers")).and_then(|h| h.as_array());

                            let mut subject = "No Subject";
                            let mut from = "Unknown Sender";
                            let mut date = "Unknown Date";

                            if let Some(hdrs) = headers {
                                for h in hdrs {
                                    let name = h.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                    let val = h.get("value").and_then(|v| v.as_str()).unwrap_or("");
                                    if name.eq_ignore_ascii_case("Subject") { subject = val; }
                                    if name.eq_ignore_ascii_case("From") { from = val; }
                                    if name.eq_ignore_ascii_case("Date") { date = val; }
                                }
                            }

                            return Some(format!("---\nFrom: {}\nDate: {}\nSubject: {}\nSnippet: {}\n", from, date, subject, snippet));
                        }
                    }
                    None
                });

                fetch_handles.push(handle);
            }
        }

        let mut results = Vec::new();
        for handle in fetch_handles {
            if let Ok(Some(res)) = handle.await {
                results.push(res);
            }
        }

        let combined_results = results.join("\n");

        println!("\x1b[32m✅ Successfully read Gmail messages\x1b[0m");

        Ok(ToolCallResult {
            tool_name: "read_gmail_oauth".to_string(),
            result: format!(
                "Retrieved {} emails:\n\n{}",
                results.len(),
                combined_results
            ),
        })
    }

    fn is_available(&self) -> bool {
        GoogleOAuthProvider::is_configured()
    }
}
