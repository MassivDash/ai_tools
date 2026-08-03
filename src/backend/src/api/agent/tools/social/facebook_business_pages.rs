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

    /// A tool using `credentials` instead of the process environment, so tests
    /// can point it at a loopback mock Graph API.
    #[cfg(test)]
    pub(crate) fn with_credentials(credentials: FacebookCredentials) -> Self {
        Self {
            credentials,
            ..Self::new()
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

    use crate::api::agent::core::types::FunctionCall;
    use crate::test_support::{MockHttpApi, MockResponse};

    /// Where `FacebookCredentials::for_test` (business biz_1) looks for Pages.
    const OWNED_PAGES_PATH: &str = "/v21.0/biz_1/owned_pages";

    fn tool_call() -> ToolCall {
        ToolCall {
            id: "call_fb_pages".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "facebook_list_business_pages".to_string(),
                arguments: "{}".to_string(),
            },
        }
    }

    fn tool_for(api: &MockHttpApi) -> FacebookBusinessPagesTool {
        FacebookBusinessPagesTool::with_credentials(FacebookCredentials::for_test(api.base_url()))
    }

    #[test]
    fn metadata_and_function_definition_describe_the_business_pages_tool() {
        let tool = FacebookBusinessPagesTool::new();
        assert_eq!(tool.metadata().id, "facebook_list_business_pages");
        assert_eq!(tool.metadata().category, ToolCategory::Social);
        assert_eq!(
            tool.metadata().tool_type,
            ToolType::FacebookBusinessPagesRead
        );

        let def = tool.get_function_definition();
        assert_eq!(def["name"], "facebook_list_business_pages");
        assert_eq!(def["parameters"]["required"], json!([]));
        assert_eq!(def["parameters"]["properties"], json!({}));
    }

    #[tokio::test]
    async fn owned_pages_are_listed_one_per_line() {
        let api = MockHttpApi::serving(
            "GET",
            OWNED_PAGES_PATH,
            MockResponse::json(json!({"data": [
                {"id": "p1", "name": "My Shop", "category": "Retail"},
                {"id": "p2"}
            ]})),
        )
        .await;

        let result = tool_for(&api)
            .execute(&tool_call())
            .await
            .expect("Listing owned pages should succeed");

        let request = api.only_request();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, OWNED_PAGES_PATH);
        assert_eq!(request.query_param("fields").as_deref(), Some(FIELDS));
        // The Page token is reused for this business-scoped call; see the caveat
        // on the tool itself.
        assert_eq!(
            request.query_param("access_token").as_deref(),
            Some("test-page-token")
        );

        assert_eq!(result.tool_name, "facebook_list_business_pages");
        assert!(result.tool_call_id.is_none());
        let lines: Vec<&str> = result.result.lines().collect();
        assert_eq!(
            lines,
            vec![
                "My Shop (Retail) | id: p1",
                "(unnamed page) (uncategorized) | id: p2",
            ]
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn a_business_with_no_pages_says_so() {
        let api = MockHttpApi::serving(
            "GET",
            OWNED_PAGES_PATH,
            MockResponse::json(json!({"data": []})),
        )
        .await;

        let result = tool_for(&api)
            .execute(&tool_call())
            .await
            .expect("An empty data array is not an error");

        assert_eq!(result.result, "No Pages found under this Business.");
        api.stop().await;
    }

    #[tokio::test]
    async fn a_permissions_error_is_surfaced() {
        let api = MockHttpApi::serving(
            "GET",
            OWNED_PAGES_PATH,
            MockResponse::error(
                403,
                r#"{"error":{"message":"(#200) requires business_management permission"}}"#,
            ),
        )
        .await;

        let error = tool_for(&api)
            .execute(&tool_call())
            .await
            .expect_err("A 403 must fail the call");

        let message = error.to_string();
        assert!(
            message.starts_with("Failed to list Facebook business pages:"),
            "{}",
            message
        );
        assert!(message.contains("business_management"), "{}", message);
        api.stop().await;
    }

    #[tokio::test]
    async fn a_missing_token_fails_before_any_request() {
        let api = MockHttpApi::serving(
            "GET",
            OWNED_PAGES_PATH,
            MockResponse::json(json!({"data": []})),
        )
        .await;
        let tokenless = FacebookBusinessPagesTool::with_credentials(
            FacebookCredentials::for_test(api.base_url()).without_access_token(),
        );

        assert_eq!(
            tokenless
                .execute(&tool_call())
                .await
                .expect_err("Without a token the call must fail")
                .to_string(),
            "FACEBOOK_PAGE_ACCESS_TOKEN environment variable not set"
        );
        assert_eq!(api.call_count(), 0, "Nothing should have reached the API");
        api.stop().await;
    }
}
