use crate::api::agent::types::{ChromaDBToolConfig, ToolCall, ToolCallResult};
use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::types::{QueryRequest, QueryResponse};
use anyhow::{Context, Result};
use serde_json::json;

/// ChromaDB tool implementation
pub struct ChromaDBTool {
    client: ChromaDBClient,
    config: ChromaDBToolConfig,
}

impl ChromaDBTool {
    /// Create a new ChromaDB tool
    pub fn new(chroma_address: &str, config: ChromaDBToolConfig) -> Result<Self> {
        let client = ChromaDBClient::new(chroma_address)
            .context("Failed to create ChromaDB client for tool")?;
        Ok(Self { client, config })
    }

    /// Execute a ChromaDB search query
    pub async fn search(&self, query: &str, n_results: Option<usize>) -> Result<String> {
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

        // Format the results as a readable string
        let formatted = format_query_results(&query_response)?;
        Ok(formatted)
    }

    /// Get the function definition for OpenAI-compatible API
    pub fn get_function_definition() -> serde_json::Value {
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
                        "description": "Number of results to return (default: 10 for technical queries, 5 for general queries, max: 20). Use more results (8-10) for technical topics, frameworks, libraries, or when you need comprehensive information. Use fewer (3-5) for simple factual questions.",
                        "minimum": 1,
                        "maximum": 20
                    }
                },
                "required": ["query"]
            }
        })
    }

    /// Execute a tool call
    pub async fn execute_tool_call(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        // Parse the function arguments
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

/// Format query results as a readable string
/// Only includes results with similarity score >= 0.5 (distance <= 0.5)
fn format_query_results(response: &QueryResponse) -> Result<String> {
    let mut formatted = String::new();
    const MIN_SIMILARITY: f64 = 0.5; // Minimum similarity threshold

    if let Some(documents) = &response.documents {
        let mut relevant_count = 0;

        for (i, doc_batch) in documents.iter().enumerate() {
            for (j, doc) in doc_batch.iter().enumerate() {
                // Check similarity score
                let should_include = if let Some(distances) = &response.distances {
                    if let Some(dist_batch) = distances.get(i) {
                        if let Some(dist) = dist_batch.get(j) {
                            let similarity = 1.0 - dist;
                            similarity >= MIN_SIMILARITY
                        } else {
                            true // Include if no distance available
                        }
                    } else {
                        true // Include if no distance batch available
                    }
                } else {
                    true // Include if no distances available
                };

                if should_include {
                    relevant_count += 1;
                    formatted.push_str(&format!("=== Document {} ===\n", relevant_count));
                    formatted.push_str(doc);
                    formatted.push_str("\n\n");
                    // Note: Similarity scores are used internally for filtering but not shown to the agent
                }
            }
        }

        if relevant_count == 0 {
            formatted.push_str("No relevant documents found. The search query did not match any documents with sufficient similarity (minimum 0.5 similarity required).");
        }
    } else {
        formatted.push_str("No documents found in the collection.");
    }

    Ok(formatted)
}
