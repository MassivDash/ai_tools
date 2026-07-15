use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::types::{Collection, QueryRequest};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use std::path::Path;
use std::process::Command;

/// ChromaDB tool implementation combining search and read actions
pub struct ChromaDBTool {
    client: ChromaDBClient,
    available_collections: Vec<Collection>,
    metadata: ToolMetadata,
}

impl ChromaDBTool {
    /// Create a new ChromaDB tool
    pub fn new(chroma_address: &str, available_collections: Vec<Collection>) -> Result<Self> {
        let client = ChromaDBClient::new(chroma_address)
            .context("Failed to create ChromaDB client for tool")?;

        let metadata = ToolMetadata {
            id: "1".to_string(),
            name: "Knowledge Base".to_string(),
            description: "Search knowledge base for information and read full documents"
                .to_string(),
            category: ToolCategory::Database,
            tool_type: ToolType::ChromaDB,
        };

        Ok(Self {
            client,
            available_collections,
            metadata,
        })
    }

    /// Execute a ChromaDB search query (internal method)
    async fn search(
        &self,
        collection_name: &str,
        query: &str,
        n_results: Option<usize>,
    ) -> Result<String> {
        let query_request = QueryRequest {
            collection: collection_name.to_string(),
            query_texts: vec![query.to_string()],
            n_results: n_results.or(Some(10)), // Default to 10 for better coverage
            where_clause: None,
        };

        let collection_info = self
            .available_collections
            .iter()
            .find(|c| c.name == collection_name)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Collection '{}' not found in available collections",
                    collection_name
                )
            })?;

        let embedding_model = collection_info
            .metadata
            .as_ref()
            .and_then(|m| m.get("embedding_model"))
            .map(|s| s.to_string())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Collection '{}' does not have an 'embedding_model' attached to its metadata. The agent cannot determine which model to use for queries.",
                    collection_name
                )
            })?;

        // Use the dynamically determined embedding model
        let query_response = self
            .client
            .query(query_request, &embedding_model)
            .await
            .context("Failed to execute ChromaDB query")?;

        // Format results: filter by cosine distance
        // For cosine distance: 0.0 = identical, 1.0 = orthogonal, 2.0 = opposite
        // We increase this to 1.2 to accommodate models that naturally have higher base distances
        const MAX_COSINE_DISTANCE: f64 = 1.2;

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

                        let metadata_str = query_response
                            .metadatas
                            .as_ref()
                            .and_then(|batch| batch.get(i))
                            .and_then(|docs| docs.get(j))
                            .map(|m| serde_json::to_string_pretty(m).unwrap_or_default())
                            .unwrap_or_else(|| "{}".to_string());

                        let dist_str = query_response
                            .distances
                            .as_ref()
                            .and_then(|dists| dists.get(i))
                            .and_then(|batch| batch.get(j))
                            .map(|&d| format!("{:.4}", d))
                            .unwrap_or_else(|| "N/A".to_string());

                        formatted.push_str(&format!(
                            "=== Document {} (Distance: {}) ===\nMetadata:\n{}\n\nContent:\n{}\n\n",
                            count, dist_str, metadata_str, doc
                        ));
                    }
                }
            }

            if count == 0 {
                formatted.push_str("No relevant documents found (similarity threshold: 1.2).");
            }
        } else {
            formatted.push_str("No documents found in the collection.");
        }

        Ok(formatted)
    }

    /// Read file content based on its extension
    async fn read_file(&self, filename: &str) -> Result<String> {
        // Sanitize the filename to prevent directory traversal
        let filename = Path::new(filename)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;

        let file_path = Path::new("./public/documents").join(filename);

        if !file_path.exists() {
            return Err(anyhow::anyhow!(
                "File '{}' not found. It may not have been uploaded or was deleted.",
                filename
            ));
        }

        if filename.ends_with(".pdf") {
            // First check if a converted markdown version exists
            let md_filename = format!("{}.md", filename);
            let md_file_path = Path::new("./public/documents").join(&md_filename);

            if md_file_path.exists() {
                let text = tokio::fs::read_to_string(&md_file_path)
                    .await
                    .context(format!("Failed to read markdown file: {:?}", md_file_path))?;
                return Ok(text);
            }

            // Use pdftotext to extract text from PDF
            let output = Command::new("pdftotext")
                .arg("-layout")
                .arg("-enc")
                .arg("UTF-8")
                .arg(&file_path)
                .arg("-") // Output to stdout
                .output()
                .context("Failed to execute pdftotext")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("pdftotext failed: {}", stderr));
            }

            let text =
                String::from_utf8(output.stdout).context("Invalid UTF-8 output from pdftotext")?;

            Ok(text)
        } else {
            // Treat everything else as text (md, txt, etc.)
            let text = tokio::fs::read_to_string(&file_path)
                .await
                .context(format!("Failed to read file: {:?}", file_path))?;
            Ok(text)
        }
    }
}

#[async_trait]
impl AgentTool for ChromaDBTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        let mut collections_info = String::new();
        if self.available_collections.is_empty() {
            collections_info.push_str("No collections currently available.\n");
        } else {
            for col in &self.available_collections {
                let meta_str = col
                    .metadata
                    .as_ref()
                    .map(|m| {
                        m.iter()
                            .map(|(k, v)| format!("{}: {}", k, v))
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_else(|| "None".to_string());
                collections_info
                    .push_str(&format!("- Name: '{}', Metadata: {}\n", col.name, meta_str));
            }
        }

        json!({
            "name": "chromadb_tool",
            "description": format!("Interact with the knowledge base. Available collections:\n{}\nYou can perform two actions: 'search' to find document snippets via semantic search, or 'read' to view a full document file. CRITICAL: For 'search', formulate a 'HyDE' (Hypothetical Document Embeddings) query: write a declarative sentence or a hypothetical document snippet that would contain the answer. Do NOT pass the user's raw question.", collections_info),
            "parameters": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["search", "read"],
                        "description": "The action to perform: 'search' to query a collection, 'read' to read a full document by filename."
                    },
                    "collection": {
                        "type": "string",
                        "description": "The name of the collection to search. Required if action is 'search'."
                    },
                    "query": {
                        "type": "string",
                        "description": "The search query. Required if action is 'search'. Must be a declarative sentence or hypothetical document snippet."
                    },
                    "filename": {
                        "type": "string",
                        "description": "The exact filename of the document to read (e.g. 'document.pdf'). Required if action is 'read'. Find this in the metadata returned by search."
                    },
                    "n_results": {
                        "type": "integer",
                        "description": "Number of results to return for search (default: 5).",
                        "minimum": 1,
                        "maximum": 10
                    }
                },
                "required": ["action"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse tool call arguments")?;

        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: action"))?;

        let result = match action {
            "search" => {
                let query = args.get("query").and_then(|v| v.as_str()).ok_or_else(|| {
                    anyhow::anyhow!("Missing required parameter for search: query")
                })?;

                let collection =
                    args.get("collection")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            anyhow::anyhow!("Missing required parameter for search: collection")
                        })?;

                let n_results = args
                    .get("n_results")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize);

                self.search(collection, query, n_results).await?
            }
            "read" => {
                let filename = args
                    .get("filename")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        anyhow::anyhow!("Missing required parameter for read: filename")
                    })?;

                match self.read_file(filename).await {
                    Ok(content) => {
                        if content.trim().is_empty() {
                            format!("The document '{}' is empty.", filename)
                        } else {
                            // Prevent LLM context overflow by truncating massive files
                            const MAX_CHARS: usize = 20000; // Roughly 5k tokens
                            if content.len() > MAX_CHARS {
                                let mut end_idx = MAX_CHARS;
                                while !content.is_char_boundary(end_idx) && end_idx > 0 {
                                    end_idx -= 1;
                                }
                                let truncated = &content[..end_idx];
                                format!("=== Document: {} (TRUNCATED due to length) ===\n\n{}\n\n...[DOCUMENT TRUNCATED: This file is over 20,000 characters and is too large to read entirely. Use search with specific queries to find exact paragraphs.]...", filename, truncated)
                            } else {
                                format!("=== Full Document: {} ===\n\n{}", filename, content)
                            }
                        }
                    }
                    Err(e) => format!("Error reading document: {}", e),
                }
            }
            _ => {
                return Err(anyhow::anyhow!("Invalid action: {}", action));
            }
        };

        Ok(ToolCallResult {
            tool_name: "chromadb_tool".to_string(),
            result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chromadb_metadata() {
        let tool_res = ChromaDBTool::new("http://localhost:8000", vec![]);

        if let Ok(tool) = tool_res {
            let metadata = tool.metadata();
            assert_eq!(metadata.id, "1");
            assert_eq!(tool.metadata().name, "Knowledge Base");
            assert_eq!(metadata.category, ToolCategory::Database);
            assert_eq!(metadata.tool_type, ToolType::ChromaDB);
        }
    }

    #[test]
    fn test_chromadb_function_definition() {
        if let Ok(tool) = ChromaDBTool::new("http://localhost:8000", vec![]) {
            let def = tool.get_function_definition();
            assert_eq!(def["name"], "chromadb_tool");
            assert!(def["parameters"]["properties"].get("action").is_some());
        }
    }
}
