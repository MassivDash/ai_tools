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
}
