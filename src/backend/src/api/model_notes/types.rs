use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelNote {
    pub id: Option<i64>,
    pub platform: String, // "llama" or "ollama"
    pub model_name: String,
    pub model_path: Option<String>, // For llama models
    pub is_favorite: bool,
    pub is_default: bool,
    pub tags: Vec<String>,
    pub notes: Option<String>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModelNoteRequest {
    pub platform: String,
    pub model_name: String,
    pub model_path: Option<String>,
    pub is_favorite: Option<bool>,
    pub is_default: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModelNotesResponse {
    pub notes: Vec<ModelNote>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModelNoteResponse {
    pub note: ModelNote,
}
