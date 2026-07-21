use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

pub struct SystemCommandTool {
    metadata: ToolMetadata,
}

impl SystemCommandTool {
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                id: "system_command".to_string(),
                name: "System Tools".to_string(),
                tool_type: ToolType::SystemCommand,
                description: "Execute safe, everyday Linux system commands (e.g., search files, view processes, check system status).".to_string(),
                category: ToolCategory::Utility,
            },
        }
    }
}

impl Default for SystemCommandTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentTool for SystemCommandTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "system_command",
            "description": "Execute safe, everyday Linux system commands (e.g., search files, view processes, check system status).",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The specific command to run. Valid options: 'search_file', 'open_folder', 'list_top_processes', 'grep_search', 'system_status', 'network_ports'",
                        "enum": ["search_file", "open_folder", "list_top_processes", "grep_search", "system_status", "network_ports"]
                    },
                    "path": {
                        "type": "string",
                        "description": "The directory path for 'search_file', 'open_folder', or 'grep_search'. E.g., '/home/user/Documents'"
                    },
                    "query": {
                        "type": "string",
                        "description": "The search pattern for 'search_file' (file name) or 'grep_search' (text content)."
                    }
                },
                "required": ["command"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).unwrap_or(json!({}));

        let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");

        let timeout_duration = Duration::from_secs(30);

        let output = match command {
            "search_file" => {
                if query.is_empty() {
                    return Ok(ToolCallResult {
                        tool_name: self.metadata.name.clone(),
                        result: "Error: 'query' parameter is required for search_file.".to_string(),
                        tool_call_id: Some(tool_call.id.clone()),
                    });
                }
                let mut cmd = Command::new("find");
                cmd.arg(path).arg("-name").arg(query);
                process_output(timeout(timeout_duration, cmd.output()).await)
            }
            "open_folder" => {
                let mut cmd = Command::new("xdg-open");
                cmd.arg(path);
                match timeout(timeout_duration, cmd.output()).await {
                    Ok(Ok(o)) if o.status.success() => "Opened successfully.".to_string(),
                    Ok(Ok(o)) => format!("Failed to open: {}", String::from_utf8_lossy(&o.stderr)),
                    Ok(Err(e)) => format!("Failed to execute xdg-open: {}", e),
                    Err(_) => "Command timed out.".to_string(),
                }
            }
            "list_top_processes" => {
                let mut cmd = Command::new("ps");
                cmd.args(["-eo", "pid,ppid,cmd,%mem,%cpu", "--sort=-%cpu"]);

                let res = process_output(timeout(timeout_duration, cmd.output()).await);
                // Get the first 15 lines in Rust instead of shelling out to `head`
                let lines: Vec<&str> = res.lines().take(16).collect();
                lines.join("\n")
            }
            "grep_search" => {
                if query.is_empty() {
                    return Ok(ToolCallResult {
                        tool_name: self.metadata.name.clone(),
                        result: "Error: 'query' parameter is required for grep_search.".to_string(),
                        tool_call_id: Some(tool_call.id.clone()),
                    });
                }
                let mut cmd = Command::new("grep");
                cmd.arg("-rn").arg(query).arg(path);
                process_output(timeout(timeout_duration, cmd.output()).await)
            }
            "system_status" => {
                let mut df_cmd = Command::new("df");
                df_cmd.arg("-h");
                let df_out = timeout(timeout_duration, df_cmd.output()).await;

                let mut free_cmd = Command::new("free");
                free_cmd.arg("-m");
                let free_out = timeout(timeout_duration, free_cmd.output()).await;

                let mut result = String::new();
                result.push_str("--- Disk Usage ---\n");
                result.push_str(&process_output(df_out));
                result.push_str("\n--- Memory Usage ---\n");
                result.push_str(&process_output(free_out));
                result
            }
            "network_ports" => {
                let mut cmd = Command::new("ss");
                cmd.arg("-tuln");
                process_output(timeout(timeout_duration, cmd.output()).await)
            }
            _ => format!("Unknown command: {}", command),
        };

        Ok(ToolCallResult {
            tool_name: self.metadata.name.clone(),
            result: output,
            tool_call_id: Some(tool_call.id.clone()),
        })
    }
}

fn process_output(
    timeout_result: Result<std::io::Result<std::process::Output>, tokio::time::error::Elapsed>,
) -> String {
    match timeout_result {
        Ok(Ok(out)) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if out.status.success() {
                if stdout.trim().is_empty() {
                    "Command executed successfully (no output).".to_string()
                } else {
                    stdout.into_owned()
                }
            } else {
                format!("Error exit {}:\n{}", out.status.code().unwrap_or(1), stderr)
            }
        }
        Ok(Err(e)) => format!("Failed to execute command: {}", e),
        Err(_) => "Command timed out after 30 seconds.".to_string(),
    }
}
