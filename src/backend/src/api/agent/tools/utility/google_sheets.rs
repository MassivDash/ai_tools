use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::agent::tools::utility::google_oauth::GoogleOAuthProvider;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct GoogleSheetsReadTool {
    metadata: ToolMetadata,
    oauth_provider: Arc<GoogleOAuthProvider>,
}

impl GoogleSheetsReadTool {
    pub fn new(oauth_provider: Arc<GoogleOAuthProvider>) -> Self {
        Self {
            metadata: ToolMetadata {
                id: "google_sheets_read".to_string(),
                name: "Read Google Sheet".to_string(),
                description: "Read data from a Google Sheet range.".to_string(),
                category: ToolCategory::Google,
                tool_type: ToolType::GoogleSheetsRead,
            },
            oauth_provider,
        }
    }
}

#[async_trait]
impl AgentTool for GoogleSheetsReadTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "google_sheets_read",
            "description": "Read the contents of a Google Sheet range.",
            "parameters": {
                "type": "object",
                "properties": {
                    "spreadsheet_id": {
                        "type": "string",
                        "description": "The ID of the Google Sheet to read."
                    },
                    "range": {
                        "type": "string",
                        "description": "The A1 notation of the range to read (e.g. 'Sheet1!A1:D10')."
                    }
                },
                "required": ["spreadsheet_id", "range"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).unwrap_or_else(|_| json!({}));

        let spreadsheet_id = args
            .get("spreadsheet_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let range = args.get("range").and_then(|v| v.as_str()).unwrap_or("");

        if spreadsheet_id.is_empty() || range.is_empty() {
            return Ok(ToolCallResult {
                tool_call_id: None,
                tool_name: "google_sheets_read".to_string(),
                result: "Error: spreadsheet_id and range are required.".to_string(),
            });
        }

        println!(
            "\x1b[36m📊 Reading Google Sheet ID: {}, Range: {}\x1b[0m",
            spreadsheet_id, range
        );

        let access_token = self.oauth_provider.get_access_token().await?;
        let client = self.oauth_provider.http_client.clone();

        let res = client
            .get(format!(
                "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
                spreadsheet_id, range
            ))
            .bearer_auth(&access_token)
            .send()
            .await
            .context("Failed to read Google Sheet")?;

        if !res.status().is_success() {
            return Err(anyhow::anyhow!(
                "Google Sheets API failed: {}",
                res.text().await.unwrap_or_default()
            ));
        }

        let doc: serde_json::Value = res.json().await.unwrap_or_default();
        let values = doc.get("values").and_then(|v| v.as_array());

        let mut text_content = String::new();
        if let Some(rows) = values {
            for row in rows {
                if let Some(cols) = row.as_array() {
                    let col_strs: Vec<String> = cols
                        .iter()
                        .map(|c| c.as_str().unwrap_or("").to_string())
                        .collect();
                    text_content.push_str(&col_strs.join(", "));
                    text_content.push('\n');
                }
            }
        }

        if text_content.is_empty() {
            text_content = "No data found in range.".to_string();
        }

        println!("\x1b[32m✅ Successfully read Google Sheet\x1b[0m");

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "google_sheets_read".to_string(),
            result: format!("Sheet Content:\n\n{}", text_content),
        })
    }

    fn is_available(&self) -> bool {
        GoogleOAuthProvider::is_configured()
    }
}

pub struct GoogleSheetsWriteTool {
    metadata: ToolMetadata,
    oauth_provider: Arc<GoogleOAuthProvider>,
}

impl GoogleSheetsWriteTool {
    pub fn new(oauth_provider: Arc<GoogleOAuthProvider>) -> Self {
        Self {
            metadata: ToolMetadata {
                id: "google_sheets_write".to_string(),
                name: "Write Google Sheet".to_string(),
                description: "Write data to a Google Sheet range.".to_string(),
                category: ToolCategory::Google,
                tool_type: ToolType::GoogleSheetsWrite,
            },
            oauth_provider,
        }
    }
}

#[async_trait]
impl AgentTool for GoogleSheetsWriteTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "google_sheets_write",
            "description": "Write data to a Google Sheet range.",
            "parameters": {
                "type": "object",
                "properties": {
                    "spreadsheet_id": {
                        "type": "string",
                        "description": "The ID of the Google Sheet to write to."
                    },
                    "range": {
                        "type": "string",
                        "description": "The A1 notation of the range to write (e.g. 'Sheet1!A1')."
                    },
                    "values": {
                        "type": "array",
                        "items": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "description": "A 2D array of strings representing rows and columns."
                    }
                },
                "required": ["spreadsheet_id", "range", "values"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).unwrap_or_else(|_| json!({}));

        let spreadsheet_id = args
            .get("spreadsheet_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let range = args.get("range").and_then(|v| v.as_str()).unwrap_or("");
        let values = args.get("values");

        if spreadsheet_id.is_empty() || range.is_empty() || values.is_none() {
            return Ok(ToolCallResult {
                tool_call_id: None,
                tool_name: "google_sheets_write".to_string(),
                result: "Error: spreadsheet_id, range, and values are required.".to_string(),
            });
        }

        println!(
            "\x1b[36m📊 Writing to Google Sheet ID: {}, Range: {}\x1b[0m",
            spreadsheet_id, range
        );

        let access_token = self.oauth_provider.get_access_token().await?;
        let client = self.oauth_provider.http_client.clone();

        let update_body = json!({
            "range": range,
            "majorDimension": "ROWS",
            "values": values
        });

        let res = client
            .put(format!(
                "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?valueInputOption=USER_ENTERED",
                spreadsheet_id, range
            ))
            .bearer_auth(&access_token)
            .json(&update_body)
            .send()
            .await
            .context("Failed to update Google Sheet")?;

        if !res.status().is_success() {
            return Err(anyhow::anyhow!(
                "Google Sheets update API failed: {}",
                res.text().await.unwrap_or_default()
            ));
        }

        println!("\x1b[32m✅ Successfully wrote to Google Sheet\x1b[0m");

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "google_sheets_write".to_string(),
            result: format!("Successfully wrote to Sheet ID: {}", spreadsheet_id),
        })
    }

    fn is_available(&self) -> bool {
        GoogleOAuthProvider::is_configured()
    }
}
