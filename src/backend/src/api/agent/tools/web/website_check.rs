use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::markdown_utils::convert::{convert_html_to_markdown, ConversionConfig};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use url::Url;

/// Website Check tool implementation
/// Converts a URL to markdown and provides it to the LLM for analysis
pub struct WebsiteCheckTool {
    metadata: ToolMetadata,
}

impl WebsiteCheckTool {
    /// Create a new Website Check tool
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                id: "3".to_string(),
                name: "Website Reader".to_string(),
                description: "Read and analyze website content".to_string(),
                category: ToolCategory::Web,
                tool_type: ToolType::WebsiteCheck,
            },
        }
    }

    /// Fetch URL and convert to markdown (internal method)
    async fn check_website(&self, url: &str) -> Result<String> {
        // Validate URL format
        Url::parse(url).context("Invalid URL format")?;

        // Fetch HTML from the URL
        let response = reqwest::get(url).await.context("Failed to fetch URL")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch URL: HTTP {}",
                response.status()
            ));
        }

        let html = response
            .text()
            .await
            .context("Failed to read response body")?;

        // Limit response size to prevent issues (10MB max)
        const MAX_HTML_SIZE: usize = 10 * 1024 * 1024;
        if html.len() > MAX_HTML_SIZE {
            return Err(anyhow::anyhow!(
                "HTML response too large: {} bytes (max {} bytes)",
                html.len(),
                MAX_HTML_SIZE
            ));
        }

        // Build conversion config with sensible defaults for website analysis
        let config = ConversionConfig {
            extract_body: true,
            enable_preprocessing: true,
            remove_navigation: true,
            remove_forms: true, // Keep forms as they might be relevant
            preprocessing_preset: Some("aggressive".to_string()),
            follow_links: false, // Only convert the main page
        };

        // Convert HTML to Markdown
        let conversion_result = convert_html_to_markdown(&html, url, &config)
            .map_err(|e| anyhow::anyhow!("Failed to convert HTML to Markdown: {}", e))?;

        // Format the result with metadata
        let mut result = format!(
            "Website: {}\n\nMarkdown Content:\n\n{}",
            url, conversion_result.markdown
        );

        // Add link count if there are internal links
        if !conversion_result.internal_links.is_empty() {
            result.push_str(&format!(
                "\n\nFound {} internal link(s) on this page.",
                conversion_result.internal_links.len()
            ));
        }

        Ok(result)
    }
}

#[async_trait]
impl AgentTool for WebsiteCheckTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "check_website",
            "description": "Fetch a website URL, convert it to markdown, and provide the content for analysis. Use this tool when the user asks about a specific website, wants to analyze web content, check what's on a webpage, or needs information from a URL. The tool will fetch the webpage, convert it to clean markdown format, and return it for you to analyze and summarize.",
            "parameters": {
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The full URL of the website to check (must include http:// or https://)"
                    }
                },
                "required": ["url"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse tool call arguments")?;

        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: url"))?;

        println!("🌐 Checking website: {}", url);
        let result = self.check_website(url).await?;
        println!("✅ Website check completed for: {}", url);

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "check_website".to_string(),
            result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::core::types::FunctionCall;
    use crate::test_support::{MockHttpApi, MockResponse};

    /// This tool fetches whatever URL the caller passes in, so there is no API
    /// host to override: the test just points the argument at the mock.
    fn tool_call(url: &str) -> ToolCall {
        tool_call_with_arguments(&json!({"url": url}).to_string())
    }

    fn tool_call_with_arguments(arguments: &str) -> ToolCall {
        ToolCall {
            id: "call_website".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "check_website".to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    #[test]
    fn metadata_and_function_definition_describe_the_website_tool() {
        let tool = WebsiteCheckTool::new();
        assert_eq!(tool.metadata().id, "3");
        assert_eq!(tool.metadata().category, ToolCategory::Web);
        assert_eq!(tool.metadata().tool_type, ToolType::WebsiteCheck);
        assert!(tool.is_available());

        let def = tool.get_function_definition();
        assert_eq!(def["name"], "check_website");
        assert_eq!(def["parameters"]["required"], json!(["url"]));
    }

    #[tokio::test]
    async fn a_page_is_fetched_and_converted_to_markdown() {
        let api = MockHttpApi::serving(
            "GET",
            "/article",
            MockResponse::html(
                r#"<html><head><title>T</title><style>p{color:red}</style></head>
                   <body><h1>Hello</h1><p>Some <strong>body</strong> text.</p>
                   <script>alert(1)</script></body></html>"#,
            ),
        )
        .await;
        let url = api.url("/article");

        let result = WebsiteCheckTool::new()
            .execute(&tool_call(&url))
            .await
            .expect("The fetch and conversion should succeed");

        let request = api.only_request();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/article");

        assert_eq!(result.tool_name, "check_website");
        assert!(result.tool_call_id.is_none());
        assert!(result.result.starts_with(&format!("Website: {}", url)));
        assert!(result.result.contains("Markdown Content:"));
        assert!(result.result.contains("Hello"));
        assert!(result.result.contains("**body**"));
        // Script and style contents are stripped rather than converted.
        assert!(!result.result.contains("alert(1)"));
        assert!(!result.result.contains("color:red"));
        // Nothing links anywhere, so no link count is appended.
        assert!(!result.result.contains("internal link(s)"));

        api.stop().await;
    }

    #[tokio::test]
    async fn same_host_links_are_counted_in_the_summary() {
        let api = MockHttpApi::serving(
            "GET",
            "/index",
            MockResponse::html(
                r#"<html><body><p>See <a href="/other">other</a> and
                   <a href="https://example.com/away">away</a>.</p></body></html>"#,
            ),
        )
        .await;

        let result = WebsiteCheckTool::new()
            .execute(&tool_call(&api.url("/index")))
            .await
            .expect("The fetch and conversion should succeed");

        // Only the same-host link counts as internal.
        assert!(
            result
                .result
                .contains("Found 1 internal link(s) on this page."),
            "{}",
            result.result
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn an_http_error_is_reported_with_its_status() {
        let api = MockHttpApi::serving("GET", "/missing", MockResponse::error(404, "nope")).await;

        let error = WebsiteCheckTool::new()
            .execute(&tool_call(&api.url("/missing")))
            .await
            .expect_err("A 404 must fail the call");

        assert_eq!(error.to_string(), "Failed to fetch URL: HTTP 404 Not Found");
        api.stop().await;
    }

    #[tokio::test]
    async fn an_unreachable_host_is_reported_as_a_fetch_failure() {
        // Port 1 is privileged and never bound, so the connection is refused.
        let error = WebsiteCheckTool::new()
            .execute(&tool_call("http://127.0.0.1:1/"))
            .await
            .expect_err("An unreachable host must fail the call");

        assert_eq!(error.to_string(), "Failed to fetch URL");
    }

    #[tokio::test]
    async fn a_malformed_url_fails_before_any_request() {
        let error = WebsiteCheckTool::new()
            .execute(&tool_call("not-a-url"))
            .await
            .expect_err("A malformed URL must fail the call");

        assert_eq!(error.to_string(), "Invalid URL format");
    }

    #[tokio::test]
    async fn bad_arguments_fail_before_any_request() {
        let tool = WebsiteCheckTool::new();

        assert_eq!(
            tool.execute(&tool_call_with_arguments("]["))
                .await
                .expect_err("Unparseable arguments must fail")
                .to_string(),
            "Failed to parse tool call arguments"
        );
        assert_eq!(
            tool.execute(&tool_call_with_arguments(
                r#"{"address": "https://example.com"}"#
            ))
            .await
            .expect_err("A missing url must fail")
            .to_string(),
            "Missing required parameter: url"
        );
    }

    #[tokio::test]
    async fn an_oversized_page_is_refused_before_conversion() {
        // One byte over the 10 MiB cap.
        const LIMIT: usize = 10 * 1024 * 1024;
        let api =
            MockHttpApi::serving("GET", "/huge", MockResponse::html(&"x".repeat(LIMIT + 1))).await;

        let error = WebsiteCheckTool::new()
            .execute(&tool_call(&api.url("/huge")))
            .await
            .expect_err("An oversized body must fail the call");

        assert_eq!(
            error.to_string(),
            format!(
                "HTML response too large: {} bytes (max {} bytes)",
                LIMIT + 1,
                LIMIT
            )
        );
        api.stop().await;
    }
}
