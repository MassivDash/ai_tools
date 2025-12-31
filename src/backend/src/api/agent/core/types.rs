use serde::{Deserialize, Deserializer, Serialize};

/// Available tools for the agent
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    ChromaDB,

    WebsiteCheck,
    Weather,
    Currency,
    Stock,
    GitHubPublic,
    GitHubAuthenticated,
    Crypto,
    // Future tools can be added here
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentConfig {
    /// List of enabled tools
    pub enabled_tools: Vec<ToolType>,
    /// ChromaDB configuration (only used if ChromaDB tool is enabled)
    pub chromadb: Option<ChromaDBToolConfig>,
}

/// ChromaDB tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromaDBToolConfig {
    /// Collection name to use for searches
    pub collection: String,
    /// Embedding model to use for queries
    pub embedding_model: String,
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
}

/// Agent config request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfigRequest {
    pub enabled_tools: Vec<ToolType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chromadb: Option<ChromaDBToolConfig>,
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
}
