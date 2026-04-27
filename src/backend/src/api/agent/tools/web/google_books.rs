use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use std::env;

/// Google Books API tool implementation
/// Allows searching for books by title or author
pub struct GoogleBooksTool {
    metadata: ToolMetadata,
    api_key: Option<String>,
}

impl GoogleBooksTool {
    /// Create a new Google Books tool
    pub fn new() -> Self {
        // Look up the API key from environment
        let api_key = env::var("GOOGLE_BOOKS_API").ok();

        Self {
            metadata: ToolMetadata {
                id: "google_books".to_string(),
                name: "Google Books Search".to_string(),
                description: "Search for books to find ISBNs and details".to_string(),
                category: ToolCategory::Web,
                tool_type: ToolType::GoogleBooks,
            },
            api_key,
        }
    }

    /// Search Google Books API
    async fn search_books(&self, query: &str) -> Result<String> {
        let mut url = format!(
            "https://www.googleapis.com/books/v1/volumes?q={}&maxResults=20",
            urlencoding::encode(query)
        );

        // Add API key if available
        if let Some(key) = &self.api_key {
            url.push_str(&format!("&key={}", urlencoding::encode(key)));
        }

        let response = reqwest::get(&url)
            .await
            .context("Failed to search Google Books")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Google Books API error: HTTP {}",
                response.status()
            ));
        }

        let json_data: serde_json::Value =
            response.json().await.context("Failed to parse response")?;

        // Format and extract relevant information into markdown
        let mut result = String::new();
        result.push_str(&format!("### Search Results for '{}'\n\n", query));

        if let Some(items) = json_data.get("items").and_then(|i| i.as_array()) {
            if items.is_empty() {
                result.push_str("No books found matching the query.\n");
            } else {
                for (i, item) in items.iter().enumerate() {
                    let volume_info = match item.get("volumeInfo") {
                        Some(info) => info,
                        None => continue,
                    };

                    let title = volume_info
                        .get("title")
                        .and_then(|t| t.as_str())
                        .unwrap_or("Unknown Title");

                    let authors = volume_info
                        .get("authors")
                        .and_then(|a| a.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|author| author.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_else(|| "Unknown Author".to_string());

                    // Extract ISBNs
                    let mut isbns = Vec::new();
                    if let Some(identifiers) = volume_info
                        .get("industryIdentifiers")
                        .and_then(|i| i.as_array())
                    {
                        for id in identifiers {
                            if let (Some(type_), Some(identifier)) = (
                                id.get("type").and_then(|t| t.as_str()),
                                id.get("identifier").and_then(|id| id.as_str()),
                            ) {
                                isbns.push(format!("{}: {}", type_, identifier));
                            }
                        }
                    }

                    // Format Book Entry
                    result.push_str(&format!("**{}. {}**  \n", i + 1, title));
                    result.push_str(&format!("*Author(s):* {}  \n", authors));
                    if !isbns.is_empty() {
                        result.push_str(&format!("*ISBN(s):* {}  \n", isbns.join(", ")));
                    }

                    if let Some(published_date) =
                        volume_info.get("publishedDate").and_then(|d| d.as_str())
                    {
                        result.push_str(&format!("*Published:* {}  \n", published_date));
                    }

                    if let Some(page_count) = volume_info.get("pageCount").and_then(|p| p.as_i64())
                    {
                        result.push_str(&format!("*Pages:* {}  \n", page_count));
                    }

                    if let Some(description) =
                        volume_info.get("description").and_then(|d| d.as_str())
                    {
                        // Truncate description if it's too long
                        let truncated = if description.len() > 150 {
                            format!("{}...", &description[..147])
                        } else {
                            description.to_string()
                        };
                        result.push_str(&format!("*Description:* {}  \n", truncated));
                    }
                    result.push_str("\n---\n\n");
                }
            }
        } else {
            result.push_str("No items found in the response.\n");
        }

        Ok(result)
    }
}

#[async_trait]
impl AgentTool for GoogleBooksTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "search_google_books",
            "description": "Search for books using the Google Books API. Useful for finding book metadata such as Title, Author, ISBN, description, and publication date.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query (e.g., a combination of title and author, or an ISBN). Example: 'intitle:Dune inauthor:Frank Herbert' or just 'Dune Frank Herbert'"
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

        println!("📚 Searching Google Books for: {}", query);
        let result = self.search_books(query).await?;
        println!("✅ Google Books search completed for: {}", query);

        Ok(ToolCallResult {
            tool_name: "search_google_books".to_string(),
            result,
        })
    }
}
