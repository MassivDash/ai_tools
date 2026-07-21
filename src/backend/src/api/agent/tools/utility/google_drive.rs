use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::agent::tools::utility::google_oauth::GoogleOAuthProvider;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct GoogleDriveSearchTool {
    metadata: ToolMetadata,
    oauth_provider: Arc<GoogleOAuthProvider>,
}

impl GoogleDriveSearchTool {
    pub fn new(oauth_provider: Arc<GoogleOAuthProvider>) -> Self {
        Self {
            metadata: ToolMetadata {
                id: "google_drive_search".to_string(),
                name: "Search Google Drive".to_string(),
                description: "Search for files and folders in Google Drive.".to_string(),
                category: ToolCategory::Google,
                tool_type: ToolType::GoogleDriveSearch,
            },
            oauth_provider,
        }
    }
}

#[async_trait]
impl AgentTool for GoogleDriveSearchTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "google_drive_search",
            "description": "Search for files and folders in Google Drive.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query. e.g. 'name contains \"taxes\"'."
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "The maximum number of files to return. Defaults to 5."
                    }
                }
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).unwrap_or_else(|_| json!({}));

        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(5);

        println!(
            "\x1b[36m🗂️ Searching Google Drive with query: '{}'\x1b[0m",
            query
        );

        let access_token = self.oauth_provider.get_access_token().await?;
        let client = self.oauth_provider.http_client.clone();

        let mut req = client
            .get("https://www.googleapis.com/drive/v3/files")
            .bearer_auth(&access_token)
            .query(&[("pageSize", &max_results.to_string())])
            .query(&[("fields", "files(id, name, mimeType, modifiedTime)")]);

        if !query.is_empty() {
            req = req.query(&[("q", query)]);
        }

        let res = req.send().await.context("Failed to search Google Drive")?;

        if !res.status().is_success() {
            return Err(anyhow::anyhow!(
                "Google Drive API failed: {}",
                res.text().await.unwrap_or_default()
            ));
        }

        let data: serde_json::Value = res.json().await.unwrap_or_default();
        let files = data.get("files").and_then(|v| v.as_array());

        let mut results = Vec::new();
        if let Some(files_array) = files {
            for file in files_array {
                let id = file.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let name = file.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let mime_type = file.get("mimeType").and_then(|v| v.as_str()).unwrap_or("");
                let modified = file
                    .get("modifiedTime")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                results.push(format!(
                    "ID: {}\nName: {}\nType: {}\nModified: {}\n---",
                    id, name, mime_type, modified
                ));
            }
        }

        if results.is_empty() {
            return Ok(ToolCallResult {
                tool_call_id: None,
                tool_name: "google_drive_search".to_string(),
                result: "No files found.".to_string(),
            });
        }

        println!("\x1b[32m✅ Successfully searched Google Drive\x1b[0m");

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "google_drive_search".to_string(),
            result: format!("Found {} files:\n\n{}", results.len(), results.join("\n")),
        })
    }

    fn is_available(&self) -> bool {
        GoogleOAuthProvider::is_configured()
    }
}

pub struct GoogleDriveReadTool {
    metadata: ToolMetadata,
    oauth_provider: Arc<GoogleOAuthProvider>,
}

impl GoogleDriveReadTool {
    pub fn new(oauth_provider: Arc<GoogleOAuthProvider>) -> Self {
        Self {
            metadata: ToolMetadata {
                id: "google_drive_read".to_string(),
                name: "Read Google Drive File".to_string(),
                description: "Read the contents of a file in Google Drive.".to_string(),
                category: ToolCategory::Google,
                tool_type: ToolType::GoogleDriveRead,
            },
            oauth_provider,
        }
    }
}

#[async_trait]
impl AgentTool for GoogleDriveReadTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "google_drive_read",
            "description": "Read the contents of a file in Google Drive. For Google Docs, Sheets, and Slides, it exports the content.",
            "parameters": {
                "type": "object",
                "properties": {
                    "file_id": {
                        "type": "string",
                        "description": "The ID of the file to read."
                    },
                    "mime_type": {
                        "type": "string",
                        "description": "The mimeType of the file. Required for Google Workspace documents to know how to export."
                    }
                },
                "required": ["file_id"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).unwrap_or_else(|_| json!({}));

        let file_id = args.get("file_id").and_then(|v| v.as_str()).unwrap_or("");
        let mime_type = args.get("mime_type").and_then(|v| v.as_str()).unwrap_or("");

        if file_id.is_empty() {
            return Ok(ToolCallResult {
                tool_call_id: None,
                tool_name: "google_drive_read".to_string(),
                result: "Error: file_id is required.".to_string(),
            });
        }

        println!(
            "\x1b[36m🗂️ Reading Google Drive file ID: {}\x1b[0m",
            file_id
        );

        let access_token = self.oauth_provider.get_access_token().await?;
        let client = self.oauth_provider.http_client.clone();

        let url = if mime_type.starts_with("application/vnd.google-apps") {
            let export_mime_type = if mime_type == "application/vnd.google-apps.document" {
                "text/plain"
            } else if mime_type == "application/vnd.google-apps.spreadsheet" {
                "text/csv"
            } else if mime_type == "application/vnd.google-apps.presentation" {
                "text/plain"
            } else {
                return Ok(ToolCallResult {
                    tool_call_id: None,
                    tool_name: "google_drive_read".to_string(),
                    result: format!(
                        "Error: Cannot read Workspace document of type {}",
                        mime_type
                    ),
                });
            };
            format!(
                "https://www.googleapis.com/drive/v3/files/{}/export?mimeType={}",
                file_id, export_mime_type
            )
        } else {
            format!(
                "https://www.googleapis.com/drive/v3/files/{}?alt=media",
                file_id
            )
        };

        let res = client
            .get(&url)
            .bearer_auth(&access_token)
            .send()
            .await
            .context("Failed to read Google Drive file")?;

        if !res.status().is_success() {
            return Err(anyhow::anyhow!(
                "Google Drive API read failed: {}",
                res.text().await.unwrap_or_default()
            ));
        }

        let content = res.text().await.unwrap_or_default();

        println!("\x1b[32m✅ Successfully read Google Drive file\x1b[0m");

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "google_drive_read".to_string(),
            result: format!("File Content:\n\n{}", content),
        })
    }

    fn is_available(&self) -> bool {
        GoogleOAuthProvider::is_configured()
    }
}
