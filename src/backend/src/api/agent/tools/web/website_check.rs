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
                name: "website check".to_string(),
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
            tool_name: "check_website".to_string(),
            result,
        })
    }
}
