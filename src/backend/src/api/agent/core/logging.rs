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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::core::types::{
        ContentPart, FunctionCall, ImageUrl, MessageRole, ToolCall,
    };

    /// `ConversationLogger::new` hardcodes `public/logs` relative to the crate
    /// root, so tests use a unique conversation id and remove the file again.
    struct LogFile {
        conversation_id: String,
        path: PathBuf,
    }

    impl LogFile {
        fn new(label: &str) -> Self {
            let conversation_id = format!("test-{}-{}", label, uuid::Uuid::new_v4());
            let path = PathBuf::from("public/logs").join(format!("{}.txt", conversation_id));
            Self {
                conversation_id,
                path,
            }
        }

        fn read(&self) -> String {
            fs::read_to_string(&self.path)
                .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", self.path, e))
        }

        fn exists(&self) -> bool {
            self.path.exists()
        }
    }

    impl Drop for LogFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }

    fn text_message(role: MessageRole, text: &str) -> ChatMessage {
        ChatMessage {
            role,
            content: MessageContent::Text(text.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }
    }

    #[test]
    fn test_new_builds_the_conversation_log_path() {
        let logger = ConversationLogger::new(true, "conv-path-check");
        assert_eq!(
            logger.log_path,
            PathBuf::from("public/logs/conv-path-check.txt")
        );
        assert!(
            PathBuf::from("public/logs").is_dir(),
            "enabling the logger should create the logs directory"
        );

        // Disabled loggers still know where they would write, but create nothing new
        let disabled = ConversationLogger::new(false, "conv-path-check-disabled");
        assert_eq!(
            disabled.log_path,
            PathBuf::from("public/logs/conv-path-check-disabled.txt")
        );
        assert!(!disabled.enabled);
    }

    #[test]
    fn test_disabled_logger_writes_nothing() {
        let file = LogFile::new("disabled");
        let logger = ConversationLogger::new(false, &file.conversation_id);

        logger.log("SECTION", "content");
        logger.log_raw("raw");
        logger.log_message(&text_message(MessageRole::User, "hello"));

        assert!(
            !file.exists(),
            "a disabled logger must not create {:?}",
            file.path
        );
    }

    #[test]
    fn test_log_appends_timestamped_sections() {
        let file = LogFile::new("sections");
        let logger = ConversationLogger::new(true, &file.conversation_id);

        logger.log("FIRST", "one");
        logger.log("SECOND", "two");

        let contents = file.read();
        assert!(contents.contains("=== FIRST ===\none\n"), "{}", contents);
        assert!(contents.contains("=== SECOND ===\ntwo\n"), "{}", contents);
        assert!(
            contents.find("FIRST").unwrap() < contents.find("SECOND").unwrap(),
            "sections should be appended in order: {}",
            contents
        );
        // A timestamp prefix of the form [YYYY-MM-DD HH:MM:SS.mmm] is present
        assert!(contents.contains("[20"), "{}", contents);
    }

    #[test]
    fn test_log_raw_writes_content_verbatim() {
        let file = LogFile::new("raw");
        let logger = ConversationLogger::new(true, &file.conversation_id);

        logger.log_raw("chunk-1");
        logger.log_raw("chunk-2");

        assert_eq!(file.read(), "chunk-1chunk-2");
    }

    #[test]
    fn test_log_message_records_role_content_tool_calls_reasoning_and_name() {
        let file = LogFile::new("message");
        let logger = ConversationLogger::new(true, &file.conversation_id);

        let message = ChatMessage {
            role: MessageRole::Assistant,
            content: MessageContent::Text("the answer".to_string()),
            name: Some("assistant-1".to_string()),
            tool_calls: Some(vec![ToolCall {
                id: "call_1".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "weather".to_string(),
                    arguments: "{\"city\":\"Rome\"}".to_string(),
                },
            }]),
            tool_call_id: None,
            reasoning_content: Some("because".to_string()),
        };

        logger.log_message(&message);

        let contents = file.read();
        assert!(contents.contains("=== MESSAGE ==="), "{}", contents);
        assert!(contents.contains("Role: Assistant"), "{}", contents);
        assert!(contents.contains("Content: the answer"), "{}", contents);
        assert!(
            contents.contains("Tool Calls:\n  - weather ({\"city\":\"Rome\"})"),
            "{}",
            contents
        );
        assert!(contents.contains("Reasoning: because"), "{}", contents);
        assert!(contents.contains("Name: assistant-1"), "{}", contents);
    }

    #[test]
    fn test_log_message_renders_multipart_content() {
        let file = LogFile::new("multipart");
        let logger = ConversationLogger::new(true, &file.conversation_id);

        logger.log_message(&ChatMessage {
            role: MessageRole::User,
            content: MessageContent::Parts(vec![
                ContentPart::Text {
                    text: "look".to_string(),
                },
                ContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: "http://example.com/a.png".to_string(),
                    },
                },
            ]),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        });

        let contents = file.read();
        assert!(contents.contains("Role: User"), "{}", contents);
        assert!(contents.contains("look"), "{}", contents);
        assert!(
            contents.contains("http://example.com/a.png"),
            "{}",
            contents
        );
        // No optional sections for a bare message
        assert!(!contents.contains("Tool Calls:"), "{}", contents);
        assert!(!contents.contains("Reasoning:"), "{}", contents);
        assert!(!contents.contains("Name:"), "{}", contents);
    }

    #[test]
    fn test_log_tool_result_writes_a_tool_result_section() {
        let file = LogFile::new("toolresult");
        let logger = ConversationLogger::new(true, &file.conversation_id);

        logger.log_tool_result(&ToolCallResult {
            tool_name: "weather".to_string(),
            result: "sunny".to_string(),
            tool_call_id: Some("call_1".to_string()),
        });

        let contents = file.read();
        assert!(contents.contains("=== TOOL RESULT ==="), "{}", contents);
        assert!(contents.contains("Tool: weather"), "{}", contents);
        assert!(contents.contains("Result: sunny"), "{}", contents);
    }

    #[test]
    fn test_log_survives_an_unopenable_log_path() {
        // Occupy the log path with a directory so `OpenOptions::open` fails.
        let file = LogFile::new("unopenable");
        fs::create_dir_all(&file.path).expect("Failed to create blocking directory");

        let logger = ConversationLogger::new(true, &file.conversation_id);
        logger.log("SECTION", "content");
        logger.log_raw("raw");

        assert!(
            file.path.is_dir(),
            "the blocking directory should be untouched"
        );
        let _ = fs::remove_dir_all(&file.path);
    }
}
