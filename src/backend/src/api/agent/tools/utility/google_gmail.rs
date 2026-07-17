use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::agent::tools::utility::google_oauth::GoogleOAuthProvider;
use anyhow::{Context, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use lettre::message::header::ContentType;
use lettre::Message;
use serde_json::json;
use std::sync::Arc;

pub struct GoogleGmailTool {
    metadata: ToolMetadata,
    oauth_provider: Arc<GoogleOAuthProvider>,
}

impl GoogleGmailTool {
    pub fn new(oauth_provider: Arc<GoogleOAuthProvider>) -> Self {
        Self {
            metadata: ToolMetadata {
                id: "send_gmail_oauth".to_string(),
                name: "Send Email (Gmail)".to_string(),
                description: "Send an HTML-formatted email using the user's Gmail account via Google Workspace APIs.".to_string(),
                category: ToolCategory::Utility,
                tool_type: ToolType::GoogleGmail,
            },
            oauth_provider,
        }
    }
}

#[async_trait]
impl AgentTool for GoogleGmailTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "send_gmail_oauth",
            "description": "Send an HTML-formatted email using the user's Gmail account.",
            "parameters": {
                "type": "object",
                "properties": {
                    "to": {
                        "type": "string",
                        "description": "The recipient's email address."
                    },
                    "subject": {
                        "type": "string",
                        "description": "The subject line of the email."
                    },
                    "body": {
                        "type": "string",
                        "description": "The HTML content of the email body."
                    }
                },
                "required": ["to", "subject", "body"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse arguments")?;

        let to = args
            .get("to")
            .and_then(|v| v.as_str())
            .context("Missing 'to'")?;
        let subject = args
            .get("subject")
            .and_then(|v| v.as_str())
            .context("Missing 'subject'")?;
        let body = args
            .get("body")
            .and_then(|v| v.as_str())
            .context("Missing 'body'")?;

        println!("\x1b[36m📧 Sending Gmail to: {}\x1b[0m", to);

        // Fetch access token
        let access_token = self.oauth_provider.get_access_token().await?;

        // Try to get user email address from token info, but for `users/me`, 'me' is automatically inferred.
        // Lettre builder requires a "from" address, but Gmail API will overwrite it with the authenticated user if missing or matching.
        // We'll put a placeholder or just build it without a From header, but lettre `Message::builder()` strictly requires a valid email.
        // We can just use a dummy "me@localhost" because the Gmail API uses the authenticated context.
        let email = Message::builder()
            .from("me@localhost".parse().unwrap())
            .to(to.parse().context("Invalid recipient email address")?)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(String::from(body))
            .context("Failed to build email message")?;

        let raw_email = email.formatted();
        let encoded_email = URL_SAFE.encode(raw_email);

        let res = self.oauth_provider.http_client
            .post("https://gmail.googleapis.com/upload/gmail/v1/users/me/messages/send")
            .bearer_auth(access_token)
            .json(&json!({
                "raw": encoded_email
            }))
            .send()
            .await
            .context("Failed to send request to Gmail API")?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Gmail API failed with {}: {}",
                status,
                text
            ));
        }

        println!("\x1b[32m✅ Gmail sent successfully\x1b[0m");

        Ok(ToolCallResult {
            tool_name: "send_gmail_oauth".to_string(),
            result: format!("Successfully sent email to {} via Gmail API", to),
        })
    }

    fn is_available(&self) -> bool {
        GoogleOAuthProvider::is_configured()
    }
}
