use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use std::env;

use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

struct EmailConfig {
    server: String,
    username: String,
    password: String,
    from: String,
    port: u16,
}

impl EmailConfig {
    fn from_env() -> Option<Self> {
        let server = env::var("SMTP_SERVER")
            .or_else(|_| env::var("SMTP_HOST"))
            .ok()?;
        let username = env::var("SMTP_USERNAME")
            .or_else(|_| env::var("SMTP_USER"))
            .ok()?;
        let password = env::var("SMTP_PASSWORD")
            .or_else(|_| env::var("SMTP_PASS"))
            .ok()?;
        let from = env::var("SMTP_FROM_EMAIL")
            .or_else(|_| env::var("SMTP_FROM"))
            .unwrap_or_else(|_| username.clone());
        let port: u16 = env::var("SMTP_PORT")
            .unwrap_or_else(|_| "587".to_string())
            .parse()
            .unwrap_or(587);

        Some(Self {
            server,
            username,
            password,
            from,
            port,
        })
    }
}

/// Email tool for sending emails via SMTP
pub struct EmailTool {
    metadata: ToolMetadata,
    mailer: Option<AsyncSmtpTransport<Tokio1Executor>>,
    from_address: Option<String>,
}

impl EmailTool {
    /// Create a new instance of the email tool
    pub fn new() -> Self {
        let config = EmailConfig::from_env();

        let (mailer, from_address) = if let Some(cfg) = config {
            let creds = Credentials::new(cfg.username, cfg.password);
            let mailer_result = if cfg.port == 465 {
                AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.server)
                    .map(|builder| builder.port(cfg.port).credentials(creds).build())
            } else {
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.server)
                    .map(|builder| builder.port(cfg.port).credentials(creds).build())
            };

            match mailer_result {
                Ok(m) => (Some(m), Some(cfg.from)),
                Err(e) => {
                    println!("⚠️ Failed to initialize EmailTool mailer: {}", e);
                    (None, None)
                }
            }
        } else {
            (None, None)
        };

        Self {
            metadata: ToolMetadata {
                id: "send_email".to_string(),
                name: "Send Email".to_string(),
                description: "Send an HTML-formatted email to a specified recipient. You must use standard HTML formatting. For example if sending a code snippet, format it with <pre><code>".to_string(),
                category: ToolCategory::Utility,
                tool_type: ToolType::Email,
            },
            mailer,
            from_address,
        }
    }
}

#[async_trait]
impl AgentTool for EmailTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "send_email",
            "description": "Send an HTML-formatted email to a specified recipient.",
            "parameters": {
                "type": "object",
                "properties": {
                    "to": {
                        "type": "string",
                        "description": "The email address of the recipient."
                    },
                    "subject": {
                        "type": "string",
                        "description": "The subject line of the email."
                    },
                    "body": {
                        "type": "string",
                        "description": "The HTML content of the email body. Make it look nice using standard HTML formatting."
                    }
                },
                "required": ["to", "subject", "body"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let mailer = self.mailer.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Email tool is not properly configured. Missing SMTP environment variables."
            )
        })?;

        let from_address = self.from_address.as_ref().unwrap();

        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse email tool arguments")?;

        let to = args
            .get("to")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: 'to'"))?;

        let subject = args
            .get("subject")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: 'subject'"))?;

        let body = args
            .get("body")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: 'body'"))?;

        println!("\x1b[36m📧 Sending email to: {}\x1b[0m", to);

        let email = Message::builder()
            .from(
                from_address
                    .parse()
                    .context("Invalid sender email address")?,
            )
            .to(to.parse().context("Invalid recipient email address")?)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(String::from(body))
            .context("Failed to build email message")?;

        match mailer.send(email).await {
            Ok(_) => {
                println!("\x1b[32m✅ Email sent successfully\x1b[0m");
                Ok(ToolCallResult {
                    tool_call_id: None,
                    tool_name: "send_email".to_string(),
                    result: format!("Successfully sent email to {}", to),
                })
            }
            Err(e) => {
                println!("\x1b[31m❌ Failed to send email: {}\x1b[0m", e);
                Err(anyhow::anyhow!("Failed to send email: {}", e))
            }
        }
    }

    fn is_available(&self) -> bool {
        self.mailer.is_some()
    }
}
