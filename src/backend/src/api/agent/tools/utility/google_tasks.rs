use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::agent::tools::utility::google_oauth::GoogleOAuthProvider;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct GoogleTasksReadTool {
    metadata: ToolMetadata,
    oauth_provider: Arc<GoogleOAuthProvider>,
}

impl GoogleTasksReadTool {
    pub fn new(oauth_provider: Arc<GoogleOAuthProvider>) -> Self {
        Self {
            metadata: ToolMetadata {
                id: "google_tasks_read".to_string(),
                name: "Read Google Tasks".to_string(),
                description: "Read tasks from a Google Tasks list.".to_string(),
                category: ToolCategory::Google,
                tool_type: ToolType::GoogleTasksRead,
            },
            oauth_provider,
        }
    }
}

#[async_trait]
impl AgentTool for GoogleTasksReadTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "google_tasks_read",
            "description": "Read tasks from a Google Tasks list.",
            "parameters": {
                "type": "object",
                "properties": {
                    "tasklist": {
                        "type": "string",
                        "description": "Task list identifier. Defaults to '@default'."
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "The maximum number of tasks to return. Defaults to 20."
                    }
                }
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).unwrap_or_else(|_| json!({}));

        let tasklist = args
            .get("tasklist")
            .and_then(|v| v.as_str())
            .unwrap_or("@default");
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(20);

        println!("\x1b[36m☑️ Reading Google Tasks list: {}\x1b[0m", tasklist);

        let access_token = self.oauth_provider.get_access_token().await?;
        let client = self.oauth_provider.http_client.clone();

        let res = client
            .get(format!(
                "https://tasks.googleapis.com/tasks/v1/lists/{}/tasks?maxResults={}",
                tasklist, max_results
            ))
            .bearer_auth(&access_token)
            .send()
            .await
            .context("Failed to read Google Tasks")?;

        if !res.status().is_success() {
            return Err(anyhow::anyhow!(
                "Google Tasks API failed: {}",
                res.text().await.unwrap_or_default()
            ));
        }

        let doc: serde_json::Value = res.json().await.unwrap_or_default();
        let items = doc.get("items").and_then(|v| v.as_array());

        let mut results = Vec::new();
        if let Some(tasks) = items {
            for task in tasks {
                let id = task.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let title = task.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let status = task.get("status").and_then(|v| v.as_str()).unwrap_or("");
                let notes = task.get("notes").and_then(|v| v.as_str()).unwrap_or("");

                results.push(format!(
                    "ID: {}\nTitle: {}\nStatus: {}\nNotes: {}\n---",
                    id, title, status, notes
                ));
            }
        }

        if results.is_empty() {
            return Ok(ToolCallResult {
                tool_call_id: None,
                tool_name: "google_tasks_read".to_string(),
                result: "No tasks found.".to_string(),
            });
        }

        println!("\x1b[32m✅ Successfully read Google Tasks\x1b[0m");

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "google_tasks_read".to_string(),
            result: format!("Found {} tasks:\n\n{}", results.len(), results.join("\n")),
        })
    }

    fn is_available(&self) -> bool {
        GoogleOAuthProvider::is_configured()
    }
}

pub struct GoogleTasksWriteTool {
    metadata: ToolMetadata,
    oauth_provider: Arc<GoogleOAuthProvider>,
}

impl GoogleTasksWriteTool {
    pub fn new(oauth_provider: Arc<GoogleOAuthProvider>) -> Self {
        Self {
            metadata: ToolMetadata {
                id: "google_tasks_write".to_string(),
                name: "Write Google Task".to_string(),
                description: "Create a new task in a Google Tasks list.".to_string(),
                category: ToolCategory::Google,
                tool_type: ToolType::GoogleTasksWrite,
            },
            oauth_provider,
        }
    }
}

#[async_trait]
impl AgentTool for GoogleTasksWriteTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "google_tasks_write",
            "description": "Create a new task in a Google Tasks list.",
            "parameters": {
                "type": "object",
                "properties": {
                    "tasklist": {
                        "type": "string",
                        "description": "Task list identifier. Defaults to '@default'."
                    },
                    "title": {
                        "type": "string",
                        "description": "The title of the task."
                    },
                    "notes": {
                        "type": "string",
                        "description": "Notes for the task."
                    },
                    "due": {
                        "type": "string",
                        "description": "Due date of the task (as a RFC 3339 timestamp)."
                    }
                },
                "required": ["title"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).unwrap_or_else(|_| json!({}));

        let tasklist = args
            .get("tasklist")
            .and_then(|v| v.as_str())
            .unwrap_or("@default");
        let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let notes = args.get("notes").and_then(|v| v.as_str());
        let due = args.get("due").and_then(|v| v.as_str());

        if title.is_empty() {
            return Ok(ToolCallResult {
                tool_call_id: None,
                tool_name: "google_tasks_write".to_string(),
                result: "Error: title is required.".to_string(),
            });
        }

        println!(
            "\x1b[36m☑️ Creating Google Task in list: {}\x1b[0m",
            tasklist
        );

        let access_token = self.oauth_provider.get_access_token().await?;
        let client = self.oauth_provider.http_client.clone();

        let mut body = json!({
            "title": title
        });

        if let Some(n) = notes {
            body.as_object_mut()
                .unwrap()
                .insert("notes".to_string(), json!(n));
        }
        if let Some(d) = due {
            body.as_object_mut()
                .unwrap()
                .insert("due".to_string(), json!(d));
        }

        let res = client
            .post(format!(
                "https://tasks.googleapis.com/tasks/v1/lists/{}/tasks",
                tasklist
            ))
            .bearer_auth(&access_token)
            .json(&body)
            .send()
            .await
            .context("Failed to create Google Task")?;

        if !res.status().is_success() {
            return Err(anyhow::anyhow!(
                "Google Tasks create API failed: {}",
                res.text().await.unwrap_or_default()
            ));
        }

        let doc: serde_json::Value = res.json().await.unwrap_or_default();
        let id = doc.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");

        println!("\x1b[32m✅ Successfully created Google Task\x1b[0m");

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "google_tasks_write".to_string(),
            result: format!("Successfully created Task ID: {}", id),
        })
    }

    fn is_available(&self) -> bool {
        GoogleOAuthProvider::is_configured()
    }
}
