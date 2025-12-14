use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromaDBConfig {
    pub embedding_model: String,
    pub query_model: String,
}

impl Default for ChromaDBConfig {
    fn default() -> Self {
        Self {
            embedding_model: "nomic-embed-text".to_string(),
            query_model: "nomic-embed-text".to_string(),
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRequest {
    pub embedding_model: String,
    pub query_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigUpdateResponse {
    pub success: bool,
    pub message: String,
}
