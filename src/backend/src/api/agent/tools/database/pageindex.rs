use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::pageindex::types::{PageIndexNode, PageIndexSummary};
use crate::api::shared::pdf::extract_pdf_text;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;

/// PageIndex tool: vectorless, reasoning-based RAG over hierarchical book indexes.
/// The LLM fetches a whole document's title/summary tree (`get_tree`), reasons about
/// which node(s) are relevant, then fetches that node's raw text on demand (`read_node`).
pub struct PageIndexTool {
    available_documents: Vec<PageIndexSummary>,
    metadata: ToolMetadata,
}

impl PageIndexTool {
    pub fn new(available_documents: Vec<PageIndexSummary>) -> Self {
        let metadata = ToolMetadata {
            id: "2".to_string(),
            name: "PageIndex".to_string(),
            description: "Navigate hierarchical book indexes by reasoning over section summaries"
                .to_string(),
            category: ToolCategory::Database,
            tool_type: ToolType::PageIndex,
        };

        Self {
            available_documents,
            metadata,
        }
    }

    /// Sanitize an id supplied by the LLM to prevent directory traversal.
    fn sanitize_id(id: &str) -> Result<String> {
        let sanitized = Path::new(id)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid id: {}", id))?;
        if sanitized != id {
            return Err(anyhow::anyhow!("Invalid id: {}", id));
        }
        Ok(sanitized.to_string())
    }

    /// Load and parse a document's tree.json from disk.
    async fn load_tree(&self, document_id: &str) -> Result<Vec<PageIndexNode>> {
        let doc_id = Self::sanitize_id(document_id)?;
        let tree_path = Path::new("./public/pageindex")
            .join(&doc_id)
            .join("tree.json");

        if !tree_path.exists() {
            return Err(anyhow::anyhow!(
                "No index found for document '{}'. It may not exist or may still be processing.",
                document_id
            ));
        }

        let contents = tokio::fs::read_to_string(&tree_path)
            .await
            .context("Failed to read tree.json")?;
        let nodes: Vec<PageIndexNode> =
            serde_json::from_str(&contents).context("Failed to parse tree.json")?;
        Ok(nodes)
    }

    /// Re-extract a specific node's raw text from the stored source PDF on demand.
    async fn read_node(&self, document_id: &str, node_id: &str) -> Result<String> {
        let nodes = self.load_tree(document_id).await?;
        let node = find_node(&nodes, node_id).ok_or_else(|| {
            anyhow::anyhow!(
                "Node '{}' not found in document '{}'. Use 'get_tree' to see valid node ids.",
                node_id,
                document_id
            )
        })?;

        let doc_id = Self::sanitize_id(document_id)?;
        let pdf_path = Path::new("./public/pageindex")
            .join(&doc_id)
            .join("source.pdf");

        let pdf_bytes = tokio::fs::read(&pdf_path)
            .await
            .context("Failed to read source PDF")?;

        let (text, _) = extract_pdf_text(&pdf_bytes, Some((node.page_start, node.page_end)))
            .map_err(|e| anyhow::anyhow!("Failed to extract section text: {}", e))?;

        Ok(text)
    }
}

/// Recursively search a node tree for a node with the given id.
fn find_node<'a>(nodes: &'a [PageIndexNode], id: &str) -> Option<&'a PageIndexNode> {
    for node in nodes {
        if node.id == id {
            return Some(node);
        }
        if let Some(found) = find_node(&node.children, id) {
            return Some(found);
        }
    }
    None
}

/// Render a node tree as an indented, LLM-readable text outline.
fn render_tree(nodes: &[PageIndexNode], depth: usize) -> String {
    let mut out = String::new();
    for node in nodes {
        let indent = "  ".repeat(depth);
        out.push_str(&format!(
            "{}- [{}] {} (pages {}-{}): {}\n",
            indent, node.id, node.title, node.page_start, node.page_end, node.summary
        ));
        if !node.children.is_empty() {
            out.push_str(&render_tree(&node.children, depth + 1));
        }
    }
    out
}

#[async_trait]
impl AgentTool for PageIndexTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> Value {
        let mut docs_info = String::new();
        if self.available_documents.is_empty() {
            docs_info.push_str("No PageIndex documents currently available.\n");
        } else {
            for doc in &self.available_documents {
                docs_info.push_str(&format!(
                    "- id: '{}', title: '{}' (file: {})\n",
                    doc.id, doc.title, doc.filename
                ));
            }
        }

        json!({
            "name": "pageindex_tool",
            "description": format!(
                "Navigate hierarchical book indexes (table of contents with per-section summaries) for reasoning-based RAG over technical books - no vector search or embeddings involved. Available documents:\n{}\nTwo actions: 'get_tree' returns a document's full chapter/section outline with a summary for each node (no raw text); 'read_node' fetches the raw extracted text of one specific node by id. Strategy: call 'get_tree' first to see the book's structure and section summaries, pick the most relevant node(s) by REASONING over the summaries (not keyword matching), then call 'read_node' to fetch that section's actual content. If a summary is too broad to confidently answer, drill into one of its child nodes instead of reading the whole parent.",
                docs_info
            ),
            "parameters": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["get_tree", "read_node"],
                        "description": "The action to perform: 'get_tree' to view a document's hierarchical outline with summaries, 'read_node' to read one section's raw text."
                    },
                    "document_id": {
                        "type": "string",
                        "description": "The id of the PageIndex document to use (see the list of available documents above). Required for both actions."
                    },
                    "node_id": {
                        "type": "string",
                        "description": "The id of the node (section) to read, as found in the tree returned by 'get_tree'. Required if action is 'read_node'."
                    }
                },
                "required": ["action", "document_id"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse tool call arguments")?;

        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: action"))?;

        let document_id = args
            .get("document_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: document_id"))?;

        let result = match action {
            "get_tree" => match self.load_tree(document_id).await {
                Ok(nodes) => {
                    if nodes.is_empty() {
                        format!("Document '{}' has an empty index.", document_id)
                    } else {
                        format!(
                            "=== Table of Contents: {} ===\n\n{}",
                            document_id,
                            render_tree(&nodes, 0)
                        )
                    }
                }
                Err(e) => format!("Error loading tree for document '{}': {}", document_id, e),
            },
            "read_node" => {
                let node_id = args
                    .get("node_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        anyhow::anyhow!("Missing required parameter for read_node: node_id")
                    })?;

                match self.read_node(document_id, node_id).await {
                    Ok(content) => {
                        if content.trim().is_empty() {
                            format!(
                                "Node '{}' in document '{}' has no extractable text.",
                                node_id, document_id
                            )
                        } else {
                            // Prevent LLM context overflow by truncating massive sections
                            const MAX_CHARS: usize = 20000; // Roughly 5k tokens
                            if content.len() > MAX_CHARS {
                                let mut end_idx = MAX_CHARS;
                                while !content.is_char_boundary(end_idx) && end_idx > 0 {
                                    end_idx -= 1;
                                }
                                let truncated = &content[..end_idx];
                                format!("=== Section: {} (TRUNCATED due to length) ===\n\n{}\n\n...[SECTION TRUNCATED: This section is over 20,000 characters. Consider drilling into a child node returned by get_tree for a narrower excerpt.]...", node_id, truncated)
                            } else {
                                format!("=== Section: {} ===\n\n{}", node_id, content)
                            }
                        }
                    }
                    Err(e) => format!(
                        "Error reading node '{}' in document '{}': {}",
                        node_id, document_id, e
                    ),
                }
            }
            _ => {
                return Err(anyhow::anyhow!("Invalid action: {}", action));
            }
        };

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "pageindex_tool".to_string(),
            result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pageindex_metadata() {
        let tool = PageIndexTool::new(vec![]);
        let metadata = tool.metadata();
        assert_eq!(metadata.id, "2");
        assert_eq!(metadata.name, "PageIndex");
        assert_eq!(metadata.category, ToolCategory::Database);
        assert_eq!(metadata.tool_type, ToolType::PageIndex);
    }

    #[test]
    fn test_pageindex_function_definition() {
        let tool = PageIndexTool::new(vec![]);
        let def = tool.get_function_definition();
        assert_eq!(def["name"], "pageindex_tool");
        assert!(def["parameters"]["properties"].get("action").is_some());
        assert!(def["parameters"]["properties"].get("document_id").is_some());
        assert!(def["parameters"]["properties"].get("node_id").is_some());
    }

    #[test]
    fn test_find_node_recursive() {
        let nodes = vec![PageIndexNode {
            id: "n1".to_string(),
            title: "Chapter 1".to_string(),
            page_start: 1,
            page_end: 20,
            summary: "Intro".to_string(),
            children: vec![PageIndexNode {
                id: "n2".to_string(),
                title: "Section 1.1".to_string(),
                page_start: 1,
                page_end: 10,
                summary: "Details".to_string(),
                children: vec![],
            }],
        }];

        assert!(find_node(&nodes, "n1").is_some());
        assert!(find_node(&nodes, "n2").is_some());
        assert!(find_node(&nodes, "n3").is_none());
    }

    #[test]
    fn test_render_tree_indents_children() {
        let nodes = vec![PageIndexNode {
            id: "n1".to_string(),
            title: "Chapter 1".to_string(),
            page_start: 1,
            page_end: 20,
            summary: "Intro".to_string(),
            children: vec![PageIndexNode {
                id: "n2".to_string(),
                title: "Section 1.1".to_string(),
                page_start: 1,
                page_end: 10,
                summary: "Details".to_string(),
                children: vec![],
            }],
        }];

        let rendered = render_tree(&nodes, 0);
        assert!(rendered.contains("- [n1] Chapter 1 (pages 1-20): Intro"));
        assert!(rendered.contains("  - [n2] Section 1.1 (pages 1-10): Details"));
    }

    use crate::api::agent::core::types::FunctionCall;

    /// A throwaway document directory under the hard-coded `./public/pageindex`
    /// root the tool reads from, removed again when the test ends.
    ///
    /// The path is not configurable, so this really does exercise the production
    /// lookup. Each guard uses a fresh uuid for its document id, so tests running
    /// in parallel never see each other's documents, and only that one
    /// subdirectory is ever deleted.
    struct TestDocument {
        id: String,
    }

    impl TestDocument {
        fn new() -> Self {
            let id = format!("test-{}", uuid::Uuid::new_v4());
            std::fs::create_dir_all(Path::new("./public/pageindex").join(&id))
                .expect("Failed to create the test document directory");
            Self { id }
        }

        fn dir(&self) -> std::path::PathBuf {
            Path::new("./public/pageindex").join(&self.id)
        }

        fn write_tree(&self, nodes: &[PageIndexNode]) {
            std::fs::write(
                self.dir().join("tree.json"),
                serde_json::to_string(nodes).expect("nodes should serialise"),
            )
            .expect("Failed to write tree.json");
        }

        fn write_raw_tree(&self, contents: &str) {
            std::fs::write(self.dir().join("tree.json"), contents)
                .expect("Failed to write tree.json");
        }

        fn write_source(&self, contents: &[u8]) {
            std::fs::write(self.dir().join("source.pdf"), contents)
                .expect("Failed to write source.pdf");
        }
    }

    impl Drop for TestDocument {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.dir());
        }
    }

    fn node(
        id: &str,
        title: &str,
        pages: (u32, u32),
        children: Vec<PageIndexNode>,
    ) -> PageIndexNode {
        PageIndexNode {
            id: id.to_string(),
            title: title.to_string(),
            page_start: pages.0,
            page_end: pages.1,
            summary: format!("{} summary", title),
            children,
        }
    }

    fn tool_call(arguments: &str) -> ToolCall {
        ToolCall {
            id: "call_pageindex".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "pageindex_tool".to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    #[test]
    fn the_description_lists_the_available_documents() {
        let empty = PageIndexTool::new(vec![]);
        let description = empty.get_function_definition()["description"]
            .as_str()
            .expect("a description")
            .to_string();
        assert!(description.contains("No PageIndex documents currently available."));

        let stocked = PageIndexTool::new(vec![PageIndexSummary {
            id: "doc1".to_string(),
            filename: "rust-book.pdf".to_string(),
            title: "The Rust Book".to_string(),
        }]);
        let description = stocked.get_function_definition()["description"]
            .as_str()
            .expect("a description")
            .to_string();
        assert!(description.contains("id: 'doc1', title: 'The Rust Book' (file: rust-book.pdf)"));
    }

    #[tokio::test]
    async fn get_tree_renders_the_stored_outline() {
        let document = TestDocument::new();
        document.write_tree(&[node(
            "n1",
            "Chapter 1",
            (1, 20),
            vec![node("n1.1", "Section 1.1", (1, 10), vec![])],
        )]);
        let tool = PageIndexTool::new(vec![]);

        let result = tool
            .execute(&tool_call(
                &json!({"action": "get_tree", "document_id": document.id}).to_string(),
            ))
            .await
            .expect("get_tree should succeed");

        assert_eq!(result.tool_name, "pageindex_tool");
        assert!(result.tool_call_id.is_none());
        assert!(result
            .result
            .starts_with(&format!("=== Table of Contents: {} ===", document.id)));
        assert!(result
            .result
            .contains("- [n1] Chapter 1 (pages 1-20): Chapter 1 summary"));
        assert!(result
            .result
            .contains("  - [n1.1] Section 1.1 (pages 1-10): Section 1.1 summary"));
    }

    #[tokio::test]
    async fn get_tree_reports_an_empty_index_a_missing_one_and_an_unparseable_one() {
        let empty = TestDocument::new();
        empty.write_tree(&[]);
        let broken = TestDocument::new();
        broken.write_raw_tree("[{\"id\":");
        let tool = PageIndexTool::new(vec![]);

        let get_tree = |id: String| {
            let tool = &tool;
            async move {
                tool.execute(&tool_call(
                    &json!({"action": "get_tree", "document_id": id}).to_string(),
                ))
                .await
                .expect("get_tree reports load failures in its result")
                .result
            }
        };

        assert_eq!(
            get_tree(empty.id.clone()).await,
            format!("Document '{}' has an empty index.", empty.id)
        );

        let missing = get_tree("no-such-document".to_string()).await;
        assert!(
            missing.starts_with("Error loading tree for document 'no-such-document': No index found for document 'no-such-document'."),
            "{}",
            missing
        );

        let unparseable = get_tree(broken.id.clone()).await;
        assert!(
            unparseable.contains("Failed to parse tree.json"),
            "{}",
            unparseable
        );
    }

    #[tokio::test]
    async fn a_traversing_document_id_is_rejected() {
        let tool = PageIndexTool::new(vec![]);

        let result = tool
            .execute(&tool_call(
                r#"{"action": "get_tree", "document_id": "../../etc"}"#,
            ))
            .await
            .expect("The rejection is reported in the result")
            .result;

        assert!(result.contains("Invalid id: ../../etc"), "{}", result);
    }

    #[tokio::test]
    async fn read_node_reports_an_unknown_node_and_an_unreadable_source() {
        let document = TestDocument::new();
        document.write_tree(&[node("n1", "Chapter 1", (1, 2), vec![])]);
        let tool = PageIndexTool::new(vec![]);

        // A node id that is not in the tree.
        let unknown = tool
            .execute(&tool_call(
                &json!({"action": "read_node", "document_id": document.id, "node_id": "n9"})
                    .to_string(),
            ))
            .await
            .expect("The failure is reported in the result")
            .result;
        assert!(
            unknown.contains("Node 'n9' not found in document"),
            "{}",
            unknown
        );
        assert!(unknown.contains("Use 'get_tree' to see valid node ids."));

        // The node exists but there is no source PDF next to the tree.
        let missing_pdf = tool
            .execute(&tool_call(
                &json!({"action": "read_node", "document_id": document.id, "node_id": "n1"})
                    .to_string(),
            ))
            .await
            .expect("The failure is reported in the result")
            .result;
        assert!(
            missing_pdf.contains("Failed to read source PDF"),
            "{}",
            missing_pdf
        );

        // A source that exists but is not a PDF fails in extraction instead.
        document.write_source(b"definitely not a PDF");
        let bad_pdf = tool
            .execute(&tool_call(
                &json!({"action": "read_node", "document_id": document.id, "node_id": "n1"})
                    .to_string(),
            ))
            .await
            .expect("The failure is reported in the result")
            .result;
        assert!(
            bad_pdf.contains("Failed to extract section text"),
            "{}",
            bad_pdf
        );
    }

    #[tokio::test]
    async fn bad_arguments_are_rejected() {
        let tool = PageIndexTool::new(vec![]);

        assert_eq!(
            tool.execute(&tool_call("not json"))
                .await
                .expect_err("Unparseable arguments must fail")
                .to_string(),
            "Failed to parse tool call arguments"
        );
        assert_eq!(
            tool.execute(&tool_call(r#"{"document_id": "d1"}"#))
                .await
                .expect_err("A missing action must fail")
                .to_string(),
            "Missing required parameter: action"
        );
        assert_eq!(
            tool.execute(&tool_call(r#"{"action": "get_tree"}"#))
                .await
                .expect_err("A missing document_id must fail")
                .to_string(),
            "Missing required parameter: document_id"
        );
        assert_eq!(
            tool.execute(&tool_call(
                r#"{"action": "read_node", "document_id": "d1"}"#
            ))
            .await
            .expect_err("read_node without a node_id must fail")
            .to_string(),
            "Missing required parameter for read_node: node_id"
        );
        assert_eq!(
            tool.execute(&tool_call(
                r#"{"action": "delete_tree", "document_id": "d1"}"#
            ))
            .await
            .expect_err("An unknown action must fail")
            .to_string(),
            "Invalid action: delete_tree"
        );
    }
}
