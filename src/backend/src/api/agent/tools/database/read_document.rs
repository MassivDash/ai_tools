use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use std::path::Path;
use std::process::Command;

/// Tool to read a full document from the public directory
pub struct ReadDocumentTool {
    metadata: ToolMetadata,
}

impl ReadDocumentTool {
    pub fn new() -> Self {
        let metadata = ToolMetadata {
            id: "read_document_1".to_string(),
            name: "Read Full Document".to_string(),
            description: "Read the entire contents of a document retrieved from the knowledge base"
                .to_string(),
            category: ToolCategory::Database,
            tool_type: ToolType::ReadDocument,
        };

        Self { metadata }
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
impl AgentTool for ReadDocumentTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "read_document",
            "description": "Read the entire content of a specific document from the knowledge base. Use this tool when you have found a relevant document snippet using `search_chromadb` and you need to read the entire file for more context or details. You MUST provide the exact `filename` found in the metadata of the `search_chromadb` results.",
            "parameters": {
                "type": "object",
                "properties": {
                    "filename": {
                        "type": "string",
                        "description": "The exact filename of the document to read (e.g. 'document.pdf' or 'notes.md'). You can find this in the metadata returned by search_chromadb."
                    }
                },
                "required": ["filename"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse tool call arguments")?;

        let filename = args
            .get("filename")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: filename"))?;

        let result = match self.read_file(filename).await {
            Ok(content) => {
                if content.trim().is_empty() {
                    format!("The document '{}' is empty.", filename)
                } else {
                    // Prevent LLM context overflow by truncating massive files
                    const MAX_CHARS: usize = 20000; // Roughly 5k tokens
                    if content.len() > MAX_CHARS {
                        // Find a safe character boundary to split on
                        let mut end_idx = MAX_CHARS;
                        while !content.is_char_boundary(end_idx) && end_idx > 0 {
                            end_idx -= 1;
                        }
                        let truncated = &content[..end_idx];
                        format!("=== Document: {} (TRUNCATED due to length) ===\n\n{}\n\n...[DOCUMENT TRUNCATED: This file is over 20,000 characters and is too large to read entirely. Use search_chromadb with specific, focused queries to find the exact paragraphs you need.]...", filename, truncated)
                    } else {
                        format!("=== Full Document: {} ===\n\n{}", filename, content)
                    }
                }
            }
            Err(e) => format!("Error reading document: {}", e),
        };

        Ok(ToolCallResult {
            tool_name: "read_document".to_string(),
            result,
        })
    }
}
