use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LlamaDefaultConfig {
    pub hf_model: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChromaDBDefaultConfig {
    pub embedding_model: String,
}
