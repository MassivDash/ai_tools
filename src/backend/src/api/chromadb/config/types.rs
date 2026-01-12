use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromaDBConfig {
    pub embedding_model: String,
    pub query_model: String,
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,
    #[serde(default = "default_chunk_overlap")]
    pub chunk_overlap: usize,
}

fn default_chunk_size() -> usize {
    384
}

fn default_chunk_overlap() -> usize {
    50
}

impl Default for ChromaDBConfig {
    fn default() -> Self {
        Self {
            embedding_model: "nomic-embed-text".to_string(),
            query_model: "nomic-embed-text".to_string(),
            chunk_size: default_chunk_size(),
            chunk_overlap: default_chunk_overlap(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub size: Option<String>,
    pub modified: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub embedding_model: String,
    pub query_model: String,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRequest {
    pub embedding_model: String,
    pub query_model: Option<String>,
    pub chunk_size: Option<usize>,
    pub chunk_overlap: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigUpdateResponse {
    pub success: bool,
    pub message: String,
}
