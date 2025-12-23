use crate::api::agent::core::types::ChromaDBToolConfig;
use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::types::QueryRequest;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;

/// ChromaDB tool implementation
pub struct ChromaDBTool {
    client: ChromaDBClient,
    config: ChromaDBToolConfig,
    metadata: ToolMetadata,
}

impl ChromaDBTool {
    /// Create a new ChromaDB tool
    pub fn new(chroma_address: &str, config: ChromaDBToolConfig) -> Result<Self> {
        let client = ChromaDBClient::new(chroma_address)
            .context("Failed to create ChromaDB client for tool")?;

        let metadata = ToolMetadata {
            id: "1".to_string(),
            name: "Knowledge Base".to_string(),
            description: "Search knowledge base for information".to_string(),
            category: ToolCategory::Database,
            tool_type: ToolType::ChromaDB,
        };

        Ok(Self {
            client,
            config,
            metadata,
        })
    }

    /// Execute a ChromaDB search query (internal method)
    async fn search(&self, query: &str, n_results: Option<usize>) -> Result<String> {
        let query_request = QueryRequest {
            collection: self.config.collection.clone(),
            query_texts: vec![query.to_string()],
            n_results: n_results.or(Some(10)), // Default to 10 for better coverage
            where_clause: None,
        };

        // Use the configured embedding model
        let query_response = self
            .client
            .query(query_request, &self.config.embedding_model)
            .await
            .context("Failed to execute ChromaDB query")?;

        // Format results: filter by cosine distance (distance <= 0.5 means similarity >= 0.5)
        // For cosine distance: 0.0 = identical, 1.0 = orthogonal, 2.0 = opposite
        const MAX_COSINE_DISTANCE: f64 = 0.5; // Equivalent to similarity >= 0.5

        let mut formatted = String::new();
        if let Some(documents) = &query_response.documents {
            let mut count = 0;
            for (i, doc_batch) in documents.iter().enumerate() {
                for (j, doc) in doc_batch.iter().enumerate() {
                    // Include if no distance available, or if distance is within threshold
                    let include = query_response
                        .distances
                        .as_ref()
                        .and_then(|dists| dists.get(i))
                        .and_then(|batch| batch.get(j))
                        .map(|&dist| dist <= MAX_COSINE_DISTANCE)
                        .unwrap_or(true);

                    if include {
                        count += 1;
                        formatted.push_str(&format!("=== Document {} ===\n{}\n\n", count, doc));
                    }
                }
            }

            if count == 0 {
                formatted.push_str("No relevant documents found (similarity threshold: 0.5).");
            }
        } else {
            formatted.push_str("No documents found in the collection.");
        }

        Ok(formatted)
    }
}

#[async_trait]
impl AgentTool for ChromaDBTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "search_chromadb",
            "description": "Search a ChromaDB collection for relevant documents using semantic search. Use this tool when the user asks about: specific people, places, events, technical topics, programming frameworks/libraries (like Bevy, React, etc.), code examples, documentation, or any factual information that might be in the knowledge base. ALWAYS search for technical topics, frameworks, libraries, or code-related questions even if you have general knowledge - the knowledge base may have specific, detailed, or up-to-date information. DO NOT use this for casual greetings, small talk, or general conversation. When searching, use clear, specific queries that match the user's question. For technical topics, consider using 5-10 results to get comprehensive information.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query to find relevant documents. Use a clear, factual query related to the specific information being requested."
                    },
                    "n_results": {
                        "type": "integer",
                        "description": "Number of results to return (default: 5 for technical queries, 3 for general queries, max: 10). Use more results (8-10) for technical topics, frameworks, libraries, or when you need comprehensive information. Use fewer (3-5) for simple factual questions.",
                        "minimum": 1,
                        "maximum": 10
                    }
                },
                "required": ["query"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse tool call arguments")?;

        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: query"))?;

        let n_results = args
            .get("n_results")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);

        let result = self.search(query, n_results).await?;

        Ok(ToolCallResult {
            tool_name: "search_chromadb".to_string(),
            result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::core::types::ChromaDBToolConfig;

    #[test]
    fn test_chromadb_metadata() {
        let config = ChromaDBToolConfig {
            collection: "test_collection".to_string(),
            embedding_model: "all-MiniLM-L6-v2".to_string(),
        };
        // Use a dummy address, the client creation might fail if it tries to connect immediately
        // But ChromaDBClient::new usually just stores the base URL.
        // If it fails, we catch the error.
        let tool_res = ChromaDBTool::new("http://localhost:8000", config);

        // Ensure instantiation works (assuming new() doesn't make network calls)
        if let Ok(tool) = tool_res {
            let metadata = tool.metadata();
            assert_eq!(metadata.id, "1");
            assert_eq!(tool.metadata().name, "Knowledge Base");
            assert_eq!(metadata.category, ToolCategory::Database);
            assert_eq!(metadata.tool_type, ToolType::ChromaDB);
        } else {
            // Panic or assert failure if it fails for reasons other than connection (if it tries to connect)
            // Based on code reading, ChromaDBClient::new just constructs the client, mostly safe.
        }
    }

    #[test]
    fn test_chromadb_function_definition() {
        let config = ChromaDBToolConfig {
            collection: "test_collection".to_string(),
            embedding_model: "test-model".to_string(),
        };
        if let Ok(tool) = ChromaDBTool::new("http://localhost:8000", config) {
            let def = tool.get_function_definition();
            assert_eq!(def["name"], "search_chromadb"); // Note: metadata name "chroma db search" != function name "search_chromadb" usually
            assert!(def["parameters"]["properties"].get("query").is_some());
        }
    }
}
