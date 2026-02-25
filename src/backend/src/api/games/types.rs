use serde::{Deserialize, Serialize};

/// Game chat request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameChatRequest {
    pub message: String,
    pub system_prompt: String,
    // Add other fields as needed, e.g., game_id, player_id
}

/// Game chat response (streaming event)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameStreamEvent {
    #[serde(rename = "text_chunk")]
    TextChunk { text: String },
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "error")]
    Error { message: String },
}

/// Standard response for simple POSTs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct GameResponse {
    pub success: bool,
    pub message: String,
}
