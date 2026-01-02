use crate::api::agent::core::types::{ChatMessage, MessageContent, ToolCallResult};
use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

/// Logger specific for agent conversation debugging
pub struct ConversationLogger {
    enabled: bool,
    log_path: PathBuf,
}

impl ConversationLogger {
    pub fn new(enabled: bool, conversation_id: &str) -> Self {
        let mut log_path = PathBuf::from("public/logs");
        if enabled {
            // Ensure logs directory exists
            if let Err(e) = fs::create_dir_all(&log_path) {
                println!("⚠️ Failed to create logs directory: {}", e);
            }
        }

        // Use conversation_id as filename to keep all logs for one conversation in the same file
        log_path.push(format!("{}.txt", conversation_id));

        Self { enabled, log_path }
    }

    pub fn log(&self, section: &str, content: &str) {
        if !self.enabled {
            return;
        }

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_entry = format!("\n[{}] === {} ===\n{}\n", timestamp, section, content);

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
        {
            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                println!("⚠️ Failed to write to log file: {}", e);
            }
        } else {
            println!("⚠️ Failed to open log file: {:?}", self.log_path);
        }
    }

    /// Logs raw content directly to the file without headers/timestamps.
    /// Useful for streaming LLM output as it arrives.
    pub fn log_raw(&self, content: &str) {
        if !self.enabled {
            return;
        }

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
        {
            if let Err(e) = file.write_all(content.as_bytes()) {
                println!("⚠️ Failed to write to log file: {}", e);
            }
        }
    }

    pub fn log_message(&self, message: &ChatMessage) {
        if !self.enabled {
            return;
        }

        let role = format!("{:?}", message.role);
        let content = match &message.content {
            MessageContent::Text(t) => t.clone(),
            MessageContent::Parts(parts) => format!("{:?}", parts),
        };

        let mut entry = format!("Role: {}\nContent: {}", role, content);

        if let Some(tool_calls) = &message.tool_calls {
            entry.push_str("\nTool Calls:");
            for tc in tool_calls {
                entry.push_str(&format!(
                    "\n  - {} ({})",
                    tc.function.name, tc.function.arguments
                ));
            }
        }

        if let Some(reasoning) = &message.reasoning_content {
            entry.push_str(&format!("\nReasoning: {}", reasoning));
        }

        if let Some(name) = &message.name {
            entry.push_str(&format!("\nName: {}", name));
        }

        self.log("MESSAGE", &entry);
    }

    pub fn log_tool_result(&self, result: &ToolCallResult) {
        self.log(
            "TOOL RESULT",
            &format!("Tool: {}\nResult: {}", result.tool_name, result.result),
        );
    }
}
