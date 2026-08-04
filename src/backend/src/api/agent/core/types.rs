use serde::{Deserialize, Deserializer, Serialize};

/// Available tools for the agent
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    ChromaDB,
    PageIndex,
    WebsiteCheck,
    Weather,
    WeatherForecast,
    Currency,
    Stock,
    GitHubPublic,
    GitHubAuthenticated,
    Crypto,
    GoogleBooks,
    Email,
    GoogleGmail,
    GoogleCalendar,
    GoogleGmailRead,
    GoogleCalendarRead,
    AskHuman,
    GoogleDriveSearch,
    GoogleDriveRead,
    GoogleDocsRead,
    GoogleDocsWrite,
    GoogleSheetsRead,
    GoogleSheetsWrite,
    GoogleTasksRead,
    GoogleTasksWrite,
    GoogleContactsRead,
    GoogleYouTubeRead,
    GooglePlacesSearch,
    BlueskyPost,
    FacebookPost,
    FacebookPostsRead,
    FacebookCommentsRead,
    FacebookMessagesRead,
    FacebookMessageSend,
    FacebookBusinessPagesRead,
    SystemCommand,
    // Future tools can be added here
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentConfig {
    /// List of enabled tools
    pub enabled_tools: Vec<ToolType>,
    /// Whether to enable debug logging for agent conversations
    #[serde(default)]
    pub debug_logging: bool,
}

/// Chat message role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

impl Default for MessageContent {
    fn default() -> Self {
        MessageContent::Text(String::new())
    }
}

impl MessageContent {
    pub fn is_empty(&self) -> bool {
        match self {
            MessageContent::Text(s) => s.is_empty(),
            MessageContent::Parts(parts) => parts.is_empty(),
        }
    }

    /// Returns the text content if it's a simple text message,
    /// or concatenates text parts if it's a multipart message.
    pub fn text(&self) -> String {
        match self {
            MessageContent::Text(s) => s.clone(),
            MessageContent::Parts(parts) => parts
                .iter()
                .filter_map(|p| match p {
                    ContentPart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(""),
        }
    }
}

/// Custom deserializer for MessageContent that handles null as empty text
fn deserialize_content_handling_null<'de, D>(deserializer: D) -> Result<MessageContent, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<MessageContent> = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    #[serde(default, deserialize_with = "deserialize_content_handling_null")]
    pub content: MessageContent, // Handles both string and array of parts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    // Some models include reasoning_content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
}

/// Tool call (function call)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: String, // Always "function" for OpenAI compatibility
    pub function: FunctionCall,
}

/// Function call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String, // JSON string
}

/// Tool definition for OpenAI-compatible API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String, // Always "function" for OpenAI compatibility
    pub function: FunctionDefinition,
}

/// Function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value, // JSON Schema
}

/// Chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub messages: Vec<ChatMessage>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamOptions {
    pub include_usage: bool,
}

/// Chat completion response from llama.cpp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

/// Choice in chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Agent chat request (from frontend)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentChatRequest {
    pub message: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<ToolCallResult>,
}

/// Agent chat response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentChatResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallResult>>,
}

/// Tool call result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub tool_name: String,
    pub result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

pub type ActiveGenerations = std::sync::Arc<
    std::sync::Mutex<std::collections::HashMap<String, tokio::sync::watch::Sender<bool>>>,
>;

/// Agent config request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfigRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_tools: Option<Vec<ToolType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_logging: Option<bool>,
}

/// Agent config response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfigResponse {
    pub success: bool,
    pub message: String,
}

/// Agent status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusResponse {
    pub active: bool,
    pub config: AgentConfig,
}

/// Model capabilities from llama server /props endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub vision: bool,
    pub audio: bool,
}

/// Model props response from llama server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPropsResponse {
    pub modalities: ModelCapabilities,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_alias: Option<String>,
    // We ignore other fields by using #[serde(flatten)] with a catch-all
    // But for simplicity, we'll just extract what we need
}

/// Streaming event types for agent responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentStreamEvent {
    #[serde(rename = "status")]
    Status {
        status: String, // "thinking", "calling_tool", "tool_result", "streaming", "done"
        message: Option<String>,
    },
    #[serde(rename = "tool_call")]
    ToolCall {
        tool_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        display_name: Option<String>,
        arguments: String,
        tool_call_id: String,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        display_name: Option<String>,
        success: bool,
        result: Option<String>,
    },
    #[serde(rename = "text_chunk")]
    TextChunk { text: String },
    #[serde(rename = "done")]
    Done {
        conversation_id: Option<String>,
        tool_calls: Option<Vec<ToolCallResult>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        usage: Option<Usage>,
    },
    #[serde(rename = "error")]
    Error { message: String },
}

/// Conversation summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: Option<String>,
    pub model: Option<String>,
    pub created_at: i64,
}

/// Update conversation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConversationRequest {
    pub title: String,
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_deserialize_chat_message_null_content() {
        let json = json!({
            "role": "assistant",
            "content": null
        });

        let message: ChatMessage =
            serde_json::from_value(json).expect("Failed to deserialize message with null content");
        assert_eq!(message.role, MessageRole::Assistant);
        match message.content {
            MessageContent::Text(text) => {
                assert!(text.is_empty(), "Content should be empty string")
            }
            _ => panic!("Expected MessageContent::Text"),
        }
    }

    #[test]
    fn test_deserialize_chat_message_missing_content() {
        let json = json!({
            "role": "user"
        });

        let message: ChatMessage = serde_json::from_value(json)
            .expect("Failed to deserialize message with missing content");
        assert_eq!(message.role, MessageRole::User);
        match message.content {
            MessageContent::Text(text) => {
                assert!(text.is_empty(), "Content should be empty string")
            }
            _ => panic!("Expected MessageContent::Text"),
        }
    }

    #[test]
    fn test_deserialize_chat_message_with_content_parts() {
        let json = json!({
            "role": "user",
            "content": [
                { "type": "text", "text": "what is this?" },
                { "type": "image_url", "image_url": { "url": "data:image/png;base64,AAA" } }
            ]
        });

        let message: ChatMessage =
            serde_json::from_value(json).expect("Failed to deserialize multipart message");

        match &message.content {
            MessageContent::Parts(parts) => {
                assert_eq!(parts.len(), 2);
                match &parts[1] {
                    ContentPart::ImageUrl { image_url } => {
                        assert_eq!(image_url.url, "data:image/png;base64,AAA");
                    }
                    other => panic!("Expected an image part, got {:?}", other),
                }
            }
            other => panic!("Expected MessageContent::Parts, got {:?}", other),
        }

        // Only the text parts are concatenated by `text()`
        assert_eq!(message.content.text(), "what is this?");
        assert!(!message.content.is_empty());
    }

    #[test]
    fn test_message_content_is_empty_and_default() {
        assert!(MessageContent::default().is_empty());
        assert!(MessageContent::Text(String::new()).is_empty());
        assert!(!MessageContent::Text("hi".to_string()).is_empty());
        assert!(MessageContent::Parts(vec![]).is_empty());
        assert!(!MessageContent::Parts(vec![ContentPart::Text {
            text: "hi".to_string()
        }])
        .is_empty());
    }

    #[test]
    fn test_message_content_text_concatenates_text_parts_only() {
        let content = MessageContent::Parts(vec![
            ContentPart::Text {
                text: "hello ".to_string(),
            },
            ContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: "http://example.com/a.png".to_string(),
                },
            },
            ContentPart::Text {
                text: "world".to_string(),
            },
        ]);

        assert_eq!(content.text(), "hello world");
        assert_eq!(MessageContent::Text("plain".to_string()).text(), "plain");
        assert_eq!(MessageContent::default().text(), "");
    }

    #[test]
    fn test_tool_type_uses_snake_case_wire_format() {
        assert_eq!(
            serde_json::to_value(ToolType::ChromaDB).unwrap(),
            json!("chroma_d_b")
        );
        assert_eq!(
            serde_json::to_value(ToolType::GitHubAuthenticated).unwrap(),
            json!("git_hub_authenticated")
        );
        assert_eq!(
            serde_json::from_value::<ToolType>(json!("weather_forecast")).unwrap(),
            ToolType::WeatherForecast
        );
        assert!(serde_json::from_value::<ToolType>(json!("not_a_tool")).is_err());
    }

    #[test]
    fn test_message_role_wire_format() {
        assert_eq!(
            serde_json::to_value(MessageRole::Assistant).unwrap(),
            json!("assistant")
        );
        for (raw, expected) in [
            ("system", MessageRole::System),
            ("user", MessageRole::User),
            ("assistant", MessageRole::Assistant),
            ("tool", MessageRole::Tool),
        ] {
            assert_eq!(
                serde_json::from_value::<MessageRole>(json!(raw)).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn test_agent_config_defaults_and_request_round_trip() {
        let config = AgentConfig::default();
        assert!(config.enabled_tools.is_empty());
        assert!(!config.debug_logging);

        // debug_logging is optional on the wire and defaults to false
        let parsed: AgentConfig = serde_json::from_value(json!({ "enabled_tools": ["weather"] }))
            .expect("Failed to deserialize config");
        assert_eq!(parsed.enabled_tools, vec![ToolType::Weather]);
        assert!(!parsed.debug_logging);

        // Absent request fields serialize away entirely
        let empty_request = AgentConfigRequest {
            enabled_tools: None,
            debug_logging: None,
        };
        assert_eq!(serde_json::to_value(empty_request).unwrap(), json!({}));
    }

    #[test]
    fn test_agent_stream_event_tagging() {
        let text = serde_json::to_value(AgentStreamEvent::TextChunk {
            text: "hi".to_string(),
        })
        .unwrap();
        assert_eq!(text, json!({ "type": "text_chunk", "text": "hi" }));

        let tool_result = serde_json::to_value(AgentStreamEvent::ToolResult {
            tool_name: "weather".to_string(),
            display_name: None,
            success: true,
            result: Some("sunny".to_string()),
        })
        .unwrap();
        assert_eq!(
            tool_result,
            json!({
                "type": "tool_result",
                "tool_name": "weather",
                "success": true,
                "result": "sunny"
            })
        );

        let done = serde_json::to_value(AgentStreamEvent::Done {
            conversation_id: Some("c1".to_string()),
            tool_calls: None,
            usage: Some(Usage {
                prompt_tokens: 1,
                completion_tokens: 2,
                total_tokens: 3,
            }),
        })
        .unwrap();
        assert_eq!(done["type"], "done");
        assert_eq!(done["conversation_id"], "c1");
        assert_eq!(done["usage"]["total_tokens"], 3);

        let error = serde_json::to_value(AgentStreamEvent::Error {
            message: "boom".to_string(),
        })
        .unwrap();
        assert_eq!(error, json!({ "type": "error", "message": "boom" }));

        let status = serde_json::to_value(AgentStreamEvent::Status {
            status: "thinking".to_string(),
            message: None,
        })
        .unwrap();
        assert_eq!(
            status,
            json!({ "type": "status", "status": "thinking", "message": null })
        );

        let tool_call = serde_json::to_value(AgentStreamEvent::ToolCall {
            tool_name: "weather".to_string(),
            display_name: Some("Weather".to_string()),
            arguments: "{}".to_string(),
            tool_call_id: "call_1".to_string(),
        })
        .unwrap();
        assert_eq!(tool_call["type"], "tool_call");
        assert_eq!(tool_call["display_name"], "Weather");
    }

    #[test]
    fn test_deserialize_chat_completion_response() {
        let json = json!({
            "id": "chatcmpl-1",
            "object": "chat.completion",
            "created": 1_700_000_000u64,
            "model": "test-model",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "hello",
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": { "name": "weather", "arguments": "{\"city\":\"Rome\"}" }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": { "prompt_tokens": 5, "completion_tokens": 7, "total_tokens": 12 }
        });

        let response: ChatCompletionResponse =
            serde_json::from_value(json).expect("Failed to deserialize response");

        assert_eq!(response.model, "test-model");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(
            response.choices[0].finish_reason.as_deref(),
            Some("tool_calls")
        );
        let calls = response.choices[0]
            .message
            .tool_calls
            .as_ref()
            .expect("Expected tool calls");
        assert_eq!(calls[0].function.name, "weather");
        assert_eq!(calls[0].tool_type, "function");
        assert_eq!(response.usage.expect("usage").total_tokens, 12);
    }

    #[test]
    fn test_agent_chat_request_accepts_plain_string_or_parts() {
        let plain: AgentChatRequest =
            serde_json::from_value(json!({ "message": "hello" })).expect("Failed to deserialize");
        assert_eq!(plain.message.text(), "hello");
        assert!(plain.conversation_id.is_none());
        assert!(plain.tool_result.is_none());

        let multipart: AgentChatRequest = serde_json::from_value(json!({
            "message": [{ "type": "text", "text": "hello" }],
            "conversation_id": "c1",
            "tool_result": { "tool_name": "weather", "result": "sunny" }
        }))
        .expect("Failed to deserialize");
        assert_eq!(multipart.message.text(), "hello");
        assert_eq!(multipart.conversation_id.as_deref(), Some("c1"));
        let tool_result = multipart.tool_result.expect("tool result");
        assert_eq!(tool_result.tool_name, "weather");
        assert!(tool_result.tool_call_id.is_none());
    }

    #[test]
    fn test_model_props_response_parses_modalities() {
        let props: ModelPropsResponse = serde_json::from_value(json!({
            "modalities": { "vision": true, "audio": false },
            "model_path": "/models/x.gguf",
            "ignored_extra_field": 42
        }))
        .expect("Failed to deserialize props");

        assert!(props.modalities.vision);
        assert!(!props.modalities.audio);
        assert_eq!(props.model_path.as_deref(), Some("/models/x.gguf"));
        assert!(props.model_alias.is_none());
    }

    #[test]
    fn test_serialize_chat_completion_request_with_usage() {
        let request = ChatCompletionRequest {
            messages: vec![],
            model: "test-model".to_string(),
            temperature: None,
            max_tokens: None,
            tools: None,
            tool_choice: None,
            stream: Some(true),
            stream_options: Some(StreamOptions {
                include_usage: true,
            }),
        };

        let json = serde_json::to_value(request).expect("Failed to serialize request");

        assert_eq!(json["stream"], true);
        assert_eq!(json["stream_options"]["include_usage"], true);
    }
}
