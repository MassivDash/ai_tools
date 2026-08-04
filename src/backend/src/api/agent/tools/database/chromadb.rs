use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::types::{Collection, QueryRequest, QueryResponse};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use std::path::Path;
use std::process::Command;

/// Cosine distance beyond which a hit is treated as irrelevant.
///
/// For cosine distance: 0.0 = identical, 1.0 = orthogonal, 2.0 = opposite. The
/// cut-off is above 1.0 to accommodate models that naturally have higher base
/// distances.
const MAX_COSINE_DISTANCE: f64 = 1.2;

/// Render query results as the prose block handed back to the model.
///
/// Hits whose distance exceeds [`MAX_COSINE_DISTANCE`] are dropped; hits with no
/// distance at all are kept. Document numbering counts only the hits that survive
/// the filter.
fn format_search_results(query_response: &QueryResponse) -> String {
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

    formatted
}

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

        Ok(format_search_results(&query_response))
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
            tool_call_id: None,
            tool_name: "chromadb_tool".to_string(),
            result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::core::types::FunctionCall;
    use crate::test_support::lock_chroma_endpoint;
    use std::collections::HashMap;

    // Note on scope: `search` finishes by calling `ChromaDBClient::query`, which
    // generates embeddings by spawning the real `ollama` binary. The tests below
    // only exercise the checks that run before that - the collection has to be
    // known and has to advertise an embedding model - so no query is ever issued
    // and the tool's own client never talks to anything.

    /// A tool whose client points at a port nothing listens on. Every test here
    /// is expected to fail (or succeed) before the client is used, so the address
    /// only has to be parseable.
    fn tool(collections: Vec<Collection>) -> ChromaDBTool {
        let _guard = lock_chroma_endpoint();
        ChromaDBTool::new("http://127.0.0.1:1", collections)
            .expect("A parseable address should always yield a tool")
    }

    fn collection(name: &str, metadata: &[(&str, &str)]) -> Collection {
        Collection {
            id: format!("{}-id", name),
            name: name.to_string(),
            metadata: if metadata.is_empty() {
                None
            } else {
                Some(
                    metadata
                        .iter()
                        .map(|(key, value)| (key.to_string(), value.to_string()))
                        .collect::<HashMap<String, String>>(),
                )
            },
            count: Some(0),
        }
    }

    fn call(arguments: serde_json::Value) -> ToolCall {
        ToolCall {
            id: "call-1".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "chromadb_tool".to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    fn raw_call(arguments: &str) -> ToolCall {
        ToolCall {
            id: "call-1".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "chromadb_tool".to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    /// A document written into the directory `read_file` reads from, deleted when
    /// the handle is dropped. Names are unique so parallel tests cannot collide
    /// with each other or with real uploads.
    struct TempDocument {
        path: std::path::PathBuf,
    }

    impl TempDocument {
        fn new(extension: &str, contents: &str) -> Self {
            let dir = Path::new("./public/documents");
            std::fs::create_dir_all(dir).expect("Failed to create the documents directory");
            let path = dir.join(format!(
                "chromadb-tool-test-{}{}",
                uuid::Uuid::new_v4(),
                extension
            ));
            std::fs::write(&path, contents).expect("Failed to write the test document");
            Self { path }
        }

        /// A sibling file with the same stem plus `suffix`, e.g. the `.md`
        /// conversion that sits next to a PDF.
        fn sibling(&self, suffix: &str, contents: &str) -> Self {
            let path = self
                .path
                .with_file_name(format!("{}{}", self.name(), suffix));
            std::fs::write(&path, contents).expect("Failed to write the sibling document");
            Self { path }
        }

        fn name(&self) -> &str {
            self.path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("The temp document has a UTF-8 name")
        }
    }

    impl Drop for TempDocument {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    #[test]
    fn test_chromadb_metadata() {
        let tool = tool(vec![]);
        let metadata = tool.metadata();

        assert_eq!(metadata.id, "1");
        assert_eq!(metadata.name, "Knowledge Base");
        assert_eq!(
            metadata.description,
            "Search knowledge base for information and read full documents"
        );
        assert_eq!(metadata.category, ToolCategory::Database);
        assert_eq!(metadata.tool_type, ToolType::ChromaDB);
    }

    #[test]
    fn test_function_definition_says_so_when_there_are_no_collections() {
        let def = tool(vec![]).get_function_definition();

        assert_eq!(def["name"], "chromadb_tool");
        let description = def["description"].as_str().unwrap();
        assert!(description.contains("No collections currently available."));

        let properties = &def["parameters"]["properties"];
        assert_eq!(
            properties["action"]["enum"],
            serde_json::json!(["search", "read"])
        );
        for key in ["action", "collection", "query", "filename", "n_results"] {
            assert!(
                properties.get(key).is_some(),
                "expected a '{key}' parameter"
            );
        }
        assert_eq!(def["parameters"]["required"], serde_json::json!(["action"]));
    }

    #[test]
    fn test_function_definition_advertises_each_collection_and_its_metadata() {
        let def = tool(vec![
            collection("notes", &[("embedding_model", "nomic-embed-text")]),
            collection("papers", &[]),
        ])
        .get_function_definition();

        let description = def["description"].as_str().unwrap();
        assert!(
            description.contains("- Name: 'notes', Metadata: embedding_model: nomic-embed-text")
        );
        assert!(description.contains("- Name: 'papers', Metadata: None"));
        assert!(!description.contains("No collections currently available."));
    }

    #[actix_web::test]
    async fn test_execute_rejects_arguments_that_are_not_json() {
        let error = tool(vec![])
            .execute(&raw_call("not json at all"))
            .await
            .unwrap_err();

        assert!(error
            .to_string()
            .contains("Failed to parse tool call arguments"));
    }

    #[actix_web::test]
    async fn test_execute_requires_an_action() {
        let error = tool(vec![])
            .execute(&call(serde_json::json!({"query": "anything"})))
            .await
            .unwrap_err();

        assert_eq!(error.to_string(), "Missing required parameter: action");
    }

    #[actix_web::test]
    async fn test_execute_rejects_an_unknown_action() {
        let error = tool(vec![])
            .execute(&call(serde_json::json!({"action": "frobnicate"})))
            .await
            .unwrap_err();

        assert_eq!(error.to_string(), "Invalid action: frobnicate");
    }

    #[actix_web::test]
    async fn test_search_requires_a_query_and_a_collection() {
        let tool = tool(vec![collection("notes", &[])]);

        let missing_query = tool
            .execute(&call(
                serde_json::json!({"action": "search", "collection": "notes"}),
            ))
            .await
            .unwrap_err();
        assert_eq!(
            missing_query.to_string(),
            "Missing required parameter for search: query"
        );

        let missing_collection = tool
            .execute(&call(
                serde_json::json!({"action": "search", "query": "a declarative sentence"}),
            ))
            .await
            .unwrap_err();
        assert_eq!(
            missing_collection.to_string(),
            "Missing required parameter for search: collection"
        );
    }

    #[actix_web::test]
    async fn test_search_refuses_a_collection_it_was_not_told_about() {
        let error = tool(vec![collection("notes", &[])])
            .execute(&call(serde_json::json!({
                "action": "search",
                "collection": "secrets",
                "query": "a declarative sentence",
            })))
            .await
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Collection 'secrets' not found in available collections"
        );
    }

    #[actix_web::test]
    async fn test_search_refuses_a_collection_with_no_embedding_model() {
        let error = tool(vec![collection("notes", &[("owner", "alice")])])
            .execute(&call(serde_json::json!({
                "action": "search",
                "collection": "notes",
                "query": "a declarative sentence",
                "n_results": 3,
            })))
            .await
            .unwrap_err();

        assert!(
            error.to_string().contains(
                "Collection 'notes' does not have an 'embedding_model' attached to its metadata"
            ),
            "unexpected error: {error}"
        );
    }

    #[actix_web::test]
    async fn test_read_requires_a_filename() {
        let error = tool(vec![])
            .execute(&call(serde_json::json!({"action": "read"})))
            .await
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Missing required parameter for read: filename"
        );
    }

    #[actix_web::test]
    async fn test_read_returns_the_whole_document() {
        let document = TempDocument::new(".txt", "the quick brown fox");

        let result = tool(vec![])
            .execute(&call(
                serde_json::json!({"action": "read", "filename": document.name()}),
            ))
            .await
            .unwrap();

        assert_eq!(result.tool_name, "chromadb_tool");
        // The tool call id is deliberately dropped by this tool.
        assert!(result.tool_call_id.is_none());
        assert_eq!(
            result.result,
            format!(
                "=== Full Document: {} ===\n\nthe quick brown fox",
                document.name()
            )
        );
    }

    #[actix_web::test]
    async fn test_read_reports_an_empty_document() {
        let document = TempDocument::new(".md", "   \n\t ");

        let result = tool(vec![])
            .execute(&call(
                serde_json::json!({"action": "read", "filename": document.name()}),
            ))
            .await
            .unwrap();

        assert_eq!(
            result.result,
            format!("The document '{}' is empty.", document.name())
        );
    }

    #[actix_web::test]
    async fn test_read_truncates_an_oversized_document() {
        let document = TempDocument::new(".txt", &"x".repeat(20_001));

        let result = tool(vec![])
            .execute(&call(
                serde_json::json!({"action": "read", "filename": document.name()}),
            ))
            .await
            .unwrap();

        assert!(result.result.contains("(TRUNCATED due to length)"));
        assert!(result.result.contains("[DOCUMENT TRUNCATED"));
        // Exactly the 20,000 character budget made it through.
        assert!(result.result.contains(&"x".repeat(20_000)));
        assert!(!result.result.contains(&"x".repeat(20_001)));
    }

    /// Truncation must not split a multi-byte character.
    #[actix_web::test]
    async fn test_read_truncates_on_a_character_boundary() {
        // "é" is two bytes, so the 20,000 byte mark lands mid-character.
        let document = TempDocument::new(".txt", &"é".repeat(10_001));

        let result = tool(vec![])
            .execute(&call(
                serde_json::json!({"action": "read", "filename": document.name()}),
            ))
            .await
            .unwrap();

        assert!(result.result.contains("(TRUNCATED due to length)"));
        assert_eq!(result.result.matches('é').count(), 10_000);
    }

    /// A PDF is served from its pre-converted markdown sibling when one exists,
    /// without shelling out to `pdftotext`.
    #[actix_web::test]
    async fn test_read_prefers_a_converted_markdown_sibling_for_a_pdf() {
        let pdf = TempDocument::new(".pdf", "not really a pdf");
        let _markdown = pdf.sibling(".md", "# Converted\n\nBody text.");

        let result = tool(vec![])
            .execute(&call(
                serde_json::json!({"action": "read", "filename": pdf.name()}),
            ))
            .await
            .unwrap();

        assert_eq!(
            result.result,
            format!(
                "=== Full Document: {} ===\n\n# Converted\n\nBody text.",
                pdf.name()
            )
        );
    }

    /// A missing document is reported to the model as text, not as a tool error.
    #[actix_web::test]
    async fn test_read_reports_a_missing_document_without_failing() {
        let result = tool(vec![])
            .execute(&call(serde_json::json!({
                "action": "read",
                "filename": "chromadb-tool-test-definitely-absent.txt",
            })))
            .await
            .unwrap();

        assert!(result.result.starts_with("Error reading document: File "));
        assert!(result.result.contains("not found"));
    }

    /// Only the basename is ever used, so a traversal attempt cannot escape the
    /// documents directory.
    #[actix_web::test]
    async fn test_read_strips_directory_components_from_the_filename() {
        let result = tool(vec![])
            .execute(&call(serde_json::json!({
                "action": "read",
                "filename": "../../../../etc/passwd",
            })))
            .await
            .unwrap();

        assert!(result.result.contains("File 'passwd' not found"));
    }

    fn response(
        documents: Option<Vec<Vec<&str>>>,
        distances: Option<Vec<Vec<f64>>>,
        metadatas: Option<Vec<Vec<serde_json::Value>>>,
    ) -> QueryResponse {
        QueryResponse {
            ids: Vec::new(),
            distances,
            documents: documents.map(|batches| {
                batches
                    .into_iter()
                    .map(|batch| batch.into_iter().map(str::to_string).collect())
                    .collect()
            }),
            metadatas: metadatas.map(|batches| {
                batches
                    .into_iter()
                    .map(|batch| {
                        batch
                            .into_iter()
                            .map(|value| {
                                value
                                    .as_object()
                                    .expect("test metadata is an object")
                                    .iter()
                                    .map(|(key, value)| (key.clone(), value.clone()))
                                    .collect()
                            })
                            .collect()
                    })
                    .collect()
            }),
        }
    }

    #[test]
    fn test_format_search_results_renders_each_hit_with_distance_and_metadata() {
        let formatted = format_search_results(&response(
            Some(vec![vec!["first body", "second body"]]),
            Some(vec![vec![0.1234, 0.5]]),
            Some(vec![vec![
                serde_json::json!({"filename": "a.pdf"}),
                serde_json::json!({"filename": "b.pdf"}),
            ]]),
        ));

        assert!(formatted.contains("=== Document 1 (Distance: 0.1234) ==="));
        assert!(formatted.contains("=== Document 2 (Distance: 0.5000) ==="));
        assert!(formatted.contains("\"filename\": \"a.pdf\""));
        assert!(formatted.contains("Content:\nfirst body"));
        assert!(formatted.contains("Content:\nsecond body"));
    }

    #[test]
    fn test_format_search_results_drops_hits_beyond_the_distance_threshold() {
        // 1.2 is exactly on the threshold and is kept; 1.21 is not.
        let formatted = format_search_results(&response(
            Some(vec![vec!["too far", "borderline"]]),
            Some(vec![vec![1.21, 1.2]]),
            None,
        ));

        assert!(!formatted.contains("too far"));
        // The surviving hit is renumbered as the first document.
        assert!(formatted.contains("=== Document 1 (Distance: 1.2000) ==="));
        assert!(formatted.contains("Content:\nborderline"));
    }

    #[test]
    fn test_format_search_results_says_so_when_everything_is_filtered_out() {
        let formatted = format_search_results(&response(
            Some(vec![vec!["too far"]]),
            Some(vec![vec![1.9]]),
            None,
        ));

        assert_eq!(
            formatted,
            "No relevant documents found (similarity threshold: 1.2)."
        );
    }

    /// Without distances, every hit is kept and reported as `N/A`, and missing
    /// metadata renders as an empty object.
    #[test]
    fn test_format_search_results_keeps_hits_that_have_no_distance() {
        let formatted = format_search_results(&response(Some(vec![vec!["a body"]]), None, None));

        assert!(formatted.contains("=== Document 1 (Distance: N/A) ==="));
        assert!(formatted.contains("Metadata:\n{}"));
    }

    #[test]
    fn test_format_search_results_says_so_when_there_are_no_documents_at_all() {
        assert_eq!(
            format_search_results(&response(None, None, None)),
            "No documents found in the collection."
        );
    }

    #[test]
    fn test_format_search_results_numbers_hits_across_query_batches() {
        let formatted = format_search_results(&response(
            Some(vec![vec!["from first query"], vec!["from second query"]]),
            Some(vec![vec![0.1], vec![0.2]]),
            None,
        ));

        assert!(formatted.contains("=== Document 1 (Distance: 0.1000) ==="));
        assert!(formatted.contains("=== Document 2 (Distance: 0.2000) ==="));
    }

    #[actix_web::test]
    async fn test_read_rejects_a_filename_with_no_basename() {
        let result = tool(vec![])
            .execute(&call(
                serde_json::json!({"action": "read", "filename": ".."}),
            ))
            .await
            .unwrap();

        assert_eq!(result.result, "Error reading document: Invalid filename");
    }
}
