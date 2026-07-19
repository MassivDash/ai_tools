use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::agent::tools::utility::google_oauth::GoogleOAuthProvider;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct GoogleDocsReadTool {
    metadata: ToolMetadata,
    oauth_provider: Arc<GoogleOAuthProvider>,
}

impl GoogleDocsReadTool {
    pub fn new(oauth_provider: Arc<GoogleOAuthProvider>) -> Self {
        Self {
            metadata: ToolMetadata {
                id: "google_docs_read".to_string(),
                name: "Read Google Doc".to_string(),
                description: "Read the contents of a Google Doc.".to_string(),
                category: ToolCategory::Google,
                tool_type: ToolType::GoogleDocsRead,
            },
            oauth_provider,
        }
    }
}

#[async_trait]
impl AgentTool for GoogleDocsReadTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "google_docs_read",
            "description": "Read the text contents of a Google Doc.",
            "parameters": {
                "type": "object",
                "properties": {
                    "document_id": {
                        "type": "string",
                        "description": "The ID of the Google Doc to read."
                    }
                },
                "required": ["document_id"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).unwrap_or_else(|_| json!({}));

        let document_id = args
            .get("document_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if document_id.is_empty() {
            return Ok(ToolCallResult {
                tool_call_id: None,
                tool_name: "google_docs_read".to_string(),
                result: "Error: document_id is required.".to_string(),
            });
        }

        println!("\x1b[36m📝 Reading Google Doc ID: {}\x1b[0m", document_id);

        let access_token = self.oauth_provider.get_access_token().await?;
        let client = self.oauth_provider.http_client.clone();

        let res = client
            .get(format!(
                "https://docs.googleapis.com/v1/documents/{}",
                document_id
            ))
            .bearer_auth(&access_token)
            .send()
            .await
            .context("Failed to read Google Doc")?;

        if !res.status().is_success() {
            return Err(anyhow::anyhow!(
                "Google Docs API failed: {}",
                res.text().await.unwrap_or_default()
            ));
        }

        let doc: serde_json::Value = res.json().await.unwrap_or_default();
        let body = doc
            .get("body")
            .and_then(|v| v.get("content"))
            .and_then(|v| v.as_array());

        let mut text_content = String::new();
        if let Some(content_array) = body {
            for element in content_array {
                if let Some(paragraph) = element.get("paragraph") {
                    if let Some(elements) = paragraph.get("elements").and_then(|v| v.as_array()) {
                        for el in elements {
                            if let Some(text_run) = el.get("textRun") {
                                if let Some(content) =
                                    text_run.get("content").and_then(|v| v.as_str())
                                {
                                    text_content.push_str(content);
                                }
                            }
                        }
                    }
                }
            }
        }

        println!("\x1b[32m✅ Successfully read Google Doc\x1b[0m");

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "google_docs_read".to_string(),
            result: format!("Doc Content:\n\n{}", text_content),
        })
    }

    fn is_available(&self) -> bool {
        GoogleOAuthProvider::is_configured()
    }
}

pub struct GoogleDocsWriteTool {
    metadata: ToolMetadata,
    oauth_provider: Arc<GoogleOAuthProvider>,
}

impl GoogleDocsWriteTool {
    pub fn new(oauth_provider: Arc<GoogleOAuthProvider>) -> Self {
        Self {
            metadata: ToolMetadata {
                id: "google_docs_write".to_string(),
                name: "Write Google Doc".to_string(),
                description: "Create a new Google Doc or insert text into an existing one."
                    .to_string(),
                category: ToolCategory::Google,
                tool_type: ToolType::GoogleDocsWrite,
            },
            oauth_provider,
        }
    }
}

#[async_trait]
impl AgentTool for GoogleDocsWriteTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "google_docs_write",
            "description": "Create a new Google Doc (if document_id is omitted) or insert text.",
            "parameters": {
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "The title of the new document. Used only if document_id is omitted."
                    },
                    "document_id": {
                        "type": "string",
                        "description": "The ID of the document to append text to. If omitted, a new document is created."
                    },
                    "text": {
                        "type": "string",
                        "description": "The text to insert."
                    }
                },
                "required": ["text"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).unwrap_or_else(|_| json!({}));

        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled Document");
        let mut document_id = args
            .get("document_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");

        if text.is_empty() && document_id.is_empty() {
            return Ok(ToolCallResult {
                tool_call_id: None,
                tool_name: "google_docs_write".to_string(),
                result: "Error: text is required.".to_string(),
            });
        }

        let access_token = self.oauth_provider.get_access_token().await?;
        let client = self.oauth_provider.http_client.clone();

        if document_id.is_empty() {
            println!("\x1b[36m📝 Creating new Google Doc: '{}'\x1b[0m", title);

            let create_body = json!({
                "title": title
            });

            let res = client
                .post("https://docs.googleapis.com/v1/documents")
                .bearer_auth(&access_token)
                .json(&create_body)
                .send()
                .await
                .context("Failed to create Google Doc")?;

            if !res.status().is_success() {
                return Err(anyhow::anyhow!(
                    "Google Docs create API failed: {}",
                    res.text().await.unwrap_or_default()
                ));
            }

            let doc: serde_json::Value = res.json().await.unwrap_or_default();
            if let Some(id) = doc.get("documentId").and_then(|v| v.as_str()) {
                document_id = id.to_string();
            }
        }

        println!(
            "\x1b[36m📝 Writing to Google Doc ID: {}\x1b[0m",
            document_id
        );

        let update_body = json!({
            "requests": [
                {
                    "insertText": {
                        "location": {
                            "index": 1
                        },
                        "text": text
                    }
                }
            ]
        });

        let res = client
            .post(format!(
                "https://docs.googleapis.com/v1/documents/{}:batchUpdate",
                document_id
            ))
            .bearer_auth(&access_token)
            .json(&update_body)
            .send()
            .await
            .context("Failed to update Google Doc")?;

        if !res.status().is_success() {
            return Err(anyhow::anyhow!(
                "Google Docs update API failed: {}",
                res.text().await.unwrap_or_default()
            ));
        }

        println!("\x1b[32m✅ Successfully wrote to Google Doc\x1b[0m");

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "google_docs_write".to_string(),
            result: format!("Successfully wrote to Document ID: {}", document_id),
        })
    }

    fn is_available(&self) -> bool {
        GoogleOAuthProvider::is_configured()
    }
}
