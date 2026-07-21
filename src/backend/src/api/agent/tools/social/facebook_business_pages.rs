use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::agent::tools::social::facebook_common::FacebookCredentials;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

const FIELDS: &str = "id,name,category";

/// Facebook Business Pages Tool implementation
/// Lists Pages owned by the configured Meta Business (requires the
/// `business_management` permission and a FACEBOOK_BUSINESS_ID).
///
/// Caveat: this reuses FACEBOOK_PAGE_ACCESS_TOKEN. Meta's docs generally
/// expect Business Manager assets (like /owned_pages) to be queried with a
/// User or System User token tied to the Business, not a plain Page token.
/// Whether a Page token also carries business-scoped access depends on how
/// it was generated. This hasn't been verified against a live Business
/// Manager token — if it 400s with a permissions error, that token/scope
/// mismatch is the first thing to check.
pub struct FacebookBusinessPagesTool {
    metadata: ToolMetadata,
    credentials: FacebookCredentials,
}

impl FacebookBusinessPagesTool {
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                id: "facebook_list_business_pages".to_string(),
                name: "Facebook List Business Pages".to_string(),
                description: "List Facebook Pages owned by the configured Meta Business"
                    .to_string(),
                category: ToolCategory::Social,
                tool_type: ToolType::FacebookBusinessPagesRead,
            },
            credentials: FacebookCredentials::from_env(),
        }
    }

    async fn list_pages(&self) -> Result<String> {
        let business_id = self.credentials.business_id()?;
        let access_token = self.credentials.access_token()?;

        let client = Client::new();
        let url = self
            .credentials
            .graph_url(&format!("{}/owned_pages", business_id));

        let response = client
            .get(&url)
            .query(&[("fields", FIELDS), ("access_token", access_token)])
            .send()
            .await
            .context("Failed to connect to Facebook Graph API")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Failed to list Facebook business pages: {}",
                error_text
            ));
        }

        let data: serde_json::Value = response.json().await?;
        let pages = data["data"].as_array().cloned().unwrap_or_default();

        if pages.is_empty() {
            return Ok("No Pages found under this Business.".to_string());
        }

        Ok(pages
            .iter()
            .map(format_page_line)
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

/// Renders a single Graph API Page object as a one-line summary.
fn format_page_line(page: &serde_json::Value) -> String {
    let id = page["id"].as_str().unwrap_or("unknown");
    let name = page["name"].as_str().unwrap_or("(unnamed page)");
    let category = page["category"].as_str().unwrap_or("uncategorized");
    format!("{} ({}) | id: {}", name, category, id)
}

#[async_trait]
impl AgentTool for FacebookBusinessPagesTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "facebook_list_business_pages",
            "description": "List Facebook Pages owned by the configured Meta Business Manager account.",
            "parameters": {
                "type": "object",
                "properties": {},
                "required": []
            }
        })
    }

    async fn execute(&self, _tool_call: &ToolCall) -> Result<ToolCallResult> {
        println!("📘 Listing Facebook business pages...");
        let result = self.list_pages().await?;

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "facebook_list_business_pages".to_string(),
            result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_page_with_full_data() {
        let page = json!({"id": "p1", "name": "My Shop", "category": "Retail"});
        let line = format_page_line(&page);
        assert!(line.contains("My Shop"));
        assert!(line.contains("Retail"));
        assert!(line.contains("p1"));
    }

    #[test]
    fn formats_page_with_missing_category() {
        let page = json!({"id": "p2", "name": "Bare Page"});
        let line = format_page_line(&page);
        assert!(line.contains("uncategorized"));
    }
}
