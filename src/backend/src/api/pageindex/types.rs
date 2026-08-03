use serde::{Deserialize, Serialize};

/// Cheap listing entry used by the agent tool & RegisterContext (no tree/summary payload).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageIndexSummary {
    pub id: String,
    pub filename: String,
    pub title: String,
}

/// Full document record as stored/returned by the PageIndex API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageIndexDocument {
    pub id: String,
    pub filename: String,
    pub title: String,
    /// "processing" | "ready" | "error"
    pub status: String,
    pub page_count: Option<u32>,
    pub node_count: Option<u32>,
    pub created_at: i64,
    pub error: Option<String>,
}

/// A single node in the hierarchical table-of-contents tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageIndexNode {
    pub id: String,
    pub title: String,
    pub page_start: u32,
    pub page_end: u32,
    pub summary: String,
    #[serde(default)]
    pub children: Vec<PageIndexNode>,
}
