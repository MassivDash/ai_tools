use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub metadata: Option<HashMap<String, String>>,
    pub count: Option<usize>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    pub collection: String,
    pub query_texts: Vec<String>,
    pub n_results: Option<usize>,
    pub where_clause: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub ids: Vec<Vec<String>>,
    pub distances: Option<Vec<Vec<f64>>>,
    pub documents: Option<Vec<Vec<String>>>,
    pub metadatas: Option<Vec<Vec<HashMap<String, serde_json::Value>>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddDocumentsRequest {
    pub collection: String,
    pub ids: Vec<String>,
    pub documents: Vec<String>,
    pub metadatas: Option<Vec<HashMap<String, String>>>,
}

/// Distance metric options for ChromaDB collections
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DistanceMetric {
    Cosine,
    L2,
    Ip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromaDBHealthResponse {
    pub status: String,
    pub version: String,
    pub chromadb: ChromaDBStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromaDBStatus {
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromaDBResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub message: Option<String>,
}
