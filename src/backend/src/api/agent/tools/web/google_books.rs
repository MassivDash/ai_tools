use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use std::env;

/// The real Google Books volumes endpoint, used unless a test overrides it.
const GOOGLE_BOOKS_URL: &str = "https://www.googleapis.com/books/v1/volumes";

/// Google Books API tool implementation
/// Allows searching for books by title or author
pub struct GoogleBooksTool {
    metadata: ToolMetadata,
    api_key: Option<String>,
    /// Volumes endpoint to talk to. Always the real Google one in production;
    /// tests point it at a loopback mock instead.
    base_url: String,
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
                category: ToolCategory::Google,
                tool_type: ToolType::GoogleBooks,
            },
            api_key,
            base_url: GOOGLE_BOOKS_URL.to_string(),
        }
    }

    /// A tool pointed at `base_url` instead of the real Google Books endpoint, so
    /// the request/response handling can be driven without the network or the
    /// `GOOGLE_BOOKS_API` env var.
    #[cfg(test)]
    pub(crate) fn with_base_url(base_url: impl Into<String>, api_key: Option<&str>) -> Self {
        Self {
            api_key: api_key.map(|key| key.to_string()),
            base_url: base_url.into(),
            ..Self::new()
        }
    }

    /// Search Google Books API
    async fn search_books(&self, query: &str) -> Result<String> {
        let mut url = format!(
            "{}?q={}&maxResults=20",
            self.base_url,
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
            tool_call_id: None,
            tool_name: "search_google_books".to_string(),
            result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::core::types::FunctionCall;
    use crate::test_support::{MockHttpApi, MockResponse};

    /// The volumes endpoint, mirrored under the mock so the recorded path matches
    /// the real API's shape.
    const VOLUMES_PATH: &str = "/books/v1/volumes";

    fn tool_call(arguments: &str) -> ToolCall {
        ToolCall {
            id: "call_books".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "search_google_books".to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    fn tool_for(api: &MockHttpApi, api_key: Option<&str>) -> GoogleBooksTool {
        GoogleBooksTool::with_base_url(api.url(VOLUMES_PATH), api_key)
    }

    #[test]
    fn metadata_and_function_definition_describe_the_books_tool() {
        let tool = GoogleBooksTool::new();
        assert_eq!(tool.metadata().id, "google_books");
        assert_eq!(tool.metadata().category, ToolCategory::Google);
        assert_eq!(tool.metadata().tool_type, ToolType::GoogleBooks);
        // No key is required to search, so the tool is always offered.
        assert!(tool.is_available());

        let def = tool.get_function_definition();
        assert_eq!(def["name"], "search_google_books");
        assert_eq!(def["parameters"]["required"], json!(["query"]));
    }

    #[tokio::test]
    async fn a_search_sends_the_encoded_query_and_formats_every_volume() {
        let api = MockHttpApi::serving(
            "GET",
            VOLUMES_PATH,
            MockResponse::json(json!({
                "items": [
                    {
                        "volumeInfo": {
                            "title": "Dune",
                            "authors": ["Frank Herbert", "Brian Herbert"],
                            "industryIdentifiers": [
                                {"type": "ISBN_10", "identifier": "0441013597"},
                                {"type": "ISBN_13", "identifier": "9780441013593"}
                            ],
                            "publishedDate": "1965-08-01",
                            "pageCount": 412,
                            "description": "A desert planet."
                        }
                    },
                    {
                        // Only a title: authors, ISBNs, date, pages and
                        // description are all optional.
                        "volumeInfo": {"title": "Dune Messiah"}
                    },
                    {
                        // No volumeInfo at all: skipped rather than fatal.
                        "id": "broken"
                    }
                ]
            })),
        )
        .await;

        let result = tool_for(&api, Some("books-key"))
            .execute(&tool_call(r#"{"query": "intitle:Dune & sand"}"#))
            .await
            .expect("The search should succeed");

        let request = api.only_request();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, VOLUMES_PATH);
        assert_eq!(
            request.query_params(),
            vec![
                ("q".to_string(), "intitle:Dune & sand".to_string()),
                ("maxResults".to_string(), "20".to_string()),
                ("key".to_string(), "books-key".to_string()),
            ]
        );

        assert_eq!(result.tool_name, "search_google_books");
        assert!(result.tool_call_id.is_none());
        let text = result.result;
        assert!(text.starts_with("### Search Results for 'intitle:Dune & sand'"));
        assert!(text.contains("**1. Dune**"));
        assert!(text.contains("*Author(s):* Frank Herbert, Brian Herbert"));
        assert!(text.contains("*ISBN(s):* ISBN_10: 0441013597, ISBN_13: 9780441013593"));
        assert!(text.contains("*Published:* 1965-08-01"));
        assert!(text.contains("*Pages:* 412"));
        assert!(text.contains("*Description:* A desert planet."));
        // The second entry keeps its numbering and falls back for the author.
        assert!(text.contains("**2. Dune Messiah**"));
        assert!(text.contains("*Author(s):* Unknown Author"));
        // The entry without volumeInfo contributes nothing.
        assert_eq!(text.matches("---").count(), 2);

        api.stop().await;
    }

    #[tokio::test]
    async fn the_key_is_omitted_when_none_is_configured() {
        let api = MockHttpApi::serving(
            "GET",
            VOLUMES_PATH,
            MockResponse::json(json!({"items": []})),
        )
        .await;

        let result = tool_for(&api, None)
            .execute(&tool_call(r#"{"query": "rust"}"#))
            .await
            .expect("An unauthenticated search should succeed");

        let request = api.only_request();
        assert!(
            request.query_param("key").is_none(),
            "No key should be sent when none is configured: {}",
            request.query
        );
        assert!(result.result.contains("No books found matching the query."));
        api.stop().await;
    }

    #[tokio::test]
    async fn a_long_description_is_truncated_on_a_char_boundary() {
        let description = "x".repeat(400);
        let api = MockHttpApi::serving(
            "GET",
            VOLUMES_PATH,
            MockResponse::json(json!({
                "items": [{"volumeInfo": {"title": "Long", "description": description}}]
            })),
        )
        .await;

        let result = tool_for(&api, None)
            .execute(&tool_call(r#"{"query": "long"}"#))
            .await
            .expect("The search should succeed");

        assert!(result
            .result
            .contains(&format!("*Description:* {}...", "x".repeat(147))));
        api.stop().await;
    }

    #[tokio::test]
    async fn a_body_without_items_says_so() {
        let api = MockHttpApi::serving(
            "GET",
            VOLUMES_PATH,
            MockResponse::json(json!({"totalItems": 0})),
        )
        .await;

        let result = tool_for(&api, None)
            .execute(&tool_call(r#"{"query": "nothing"}"#))
            .await
            .expect("A body without items is not an error");

        assert!(result.result.contains("No items found in the response."));
        api.stop().await;
    }

    #[tokio::test]
    async fn an_http_error_is_reported_with_its_status() {
        let api = MockHttpApi::serving(
            "GET",
            VOLUMES_PATH,
            MockResponse::error(403, "dailyLimitExceeded"),
        )
        .await;

        let error = tool_for(&api, Some("k"))
            .execute(&tool_call(r#"{"query": "rust"}"#))
            .await
            .expect_err("A 403 must fail the call");

        assert_eq!(
            error.to_string(),
            "Google Books API error: HTTP 403 Forbidden"
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn malformed_json_is_reported_as_a_parse_failure() {
        let api = MockHttpApi::serving(
            "GET",
            VOLUMES_PATH,
            MockResponse::raw(200, "application/json", "{\"items\":"),
        )
        .await;

        let error = tool_for(&api, None)
            .execute(&tool_call(r#"{"query": "rust"}"#))
            .await
            .expect_err("A truncated body must fail the call");

        assert_eq!(error.to_string(), "Failed to parse response");
        api.stop().await;
    }

    #[tokio::test]
    async fn an_unreachable_api_is_reported_as_a_request_failure() {
        // Port 1 is privileged and never bound, so the connection is refused.
        let tool = GoogleBooksTool::with_base_url("http://127.0.0.1:1/books/v1/volumes", None);

        let error = tool
            .execute(&tool_call(r#"{"query": "rust"}"#))
            .await
            .expect_err("An unreachable API must fail the call");

        assert_eq!(error.to_string(), "Failed to search Google Books");
    }

    #[tokio::test]
    async fn bad_arguments_fail_before_any_request() {
        let api = MockHttpApi::serving(
            "GET",
            VOLUMES_PATH,
            MockResponse::json(json!({"items": []})),
        )
        .await;
        let tool = tool_for(&api, None);

        assert_eq!(
            tool.execute(&tool_call("nonsense"))
                .await
                .expect_err("Unparseable arguments must fail")
                .to_string(),
            "Failed to parse tool call arguments"
        );
        assert_eq!(
            tool.execute(&tool_call(r#"{"q": "wrong field name"}"#))
                .await
                .expect_err("A missing query must fail")
                .to_string(),
            "Missing required parameter: query"
        );

        assert_eq!(api.call_count(), 0, "Nothing should have reached the API");
        api.stop().await;
    }
}
