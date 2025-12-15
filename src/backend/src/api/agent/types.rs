use serde::{Deserialize, Serialize};

/// Available tools for the agent
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    ChromaDB,
    FinancialData,
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

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    #[serde(deserialize_with = "deserialize_null_as_empty_string")]
    pub content: String, // Handle null as empty string (common when tool_calls are present)
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

/// Custom deserializer to handle null content as empty string
fn deserialize_null_as_empty_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
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
    pub message: String,
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
        arguments: String,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_name: String,
        success: bool,
        result: Option<String>,
    },
    #[serde(rename = "text_chunk")]
    TextChunk { text: String },
    #[serde(rename = "done")]
    Done {
        conversation_id: Option<String>,
        tool_calls: Option<Vec<ToolCallResult>>,
    },
    #[serde(rename = "error")]
    Error { message: String },
}
