use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::agent::tools::utility::google_oauth::GoogleOAuthProvider;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct GoogleContactsReadTool {
    metadata: ToolMetadata,
    oauth_provider: Arc<GoogleOAuthProvider>,
}

impl GoogleContactsReadTool {
    pub fn new(oauth_provider: Arc<GoogleOAuthProvider>) -> Self {
        Self {
            metadata: ToolMetadata {
                id: "google_contacts_read".to_string(),
                name: "Read Google Contacts".to_string(),
                description: "Read contacts from Google using the People API.".to_string(),
                category: ToolCategory::Google,
                tool_type: ToolType::GoogleContactsRead,
            },
            oauth_provider,
        }
    }
}

#[async_trait]
impl AgentTool for GoogleContactsReadTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "google_contacts_read",
            "description": "Read contacts from Google using the People API.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Optional search query to filter contacts by name, email, etc."
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "The maximum number of contacts to return. Defaults to 10."
                    }
                }
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).unwrap_or_else(|_| json!({}));

        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(10);

        println!(
            "\x1b[36m👥 Reading Google Contacts with query: '{}'\x1b[0m",
            query
        );

        let access_token = self.oauth_provider.get_access_token().await?;
        let client = self.oauth_provider.http_client.clone();

        let url = if query.is_empty() {
            format!(
                "https://people.googleapis.com/v1/people/me/connections?personFields=names,emailAddresses,phoneNumbers&pageSize={}",
                max_results
            )
        } else {
            format!(
                "https://people.googleapis.com/v1/people:searchContacts?query={}&readMask=names,emailAddresses,phoneNumbers&pageSize={}",
                urlencoding::encode(query), max_results
            )
        };

        let res = client
            .get(&url)
            .bearer_auth(&access_token)
            .send()
            .await
            .context("Failed to read Google Contacts")?;

        if !res.status().is_success() {
            return Err(anyhow::anyhow!(
                "Google Contacts API failed: {}",
                res.text().await.unwrap_or_default()
            ));
        }

        let doc: serde_json::Value = res.json().await.unwrap_or_default();

        let mut results = Vec::new();

        // Handle both connection list and search results
        let people_list: Vec<serde_json::Value> = if query.is_empty() {
            doc.get("connections")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
        } else {
            let mut arr = Vec::new();
            if let Some(results_arr) = doc.get("results").and_then(|v| v.as_array()) {
                for r in results_arr {
                    if let Some(person) = r.get("person") {
                        arr.push(person.clone());
                    }
                }
            }
            arr
        };

        for person in people_list {
            let name = person
                .get("names")
                .and_then(|v| v.as_array())
                .and_then(|v| v.first())
                .and_then(|v| v.get("displayName"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");

            let email = person
                .get("emailAddresses")
                .and_then(|v| v.as_array())
                .and_then(|v| v.first())
                .and_then(|v| v.get("value"))
                .and_then(|v| v.as_str())
                .unwrap_or("No email");

            let phone = person
                .get("phoneNumbers")
                .and_then(|v| v.as_array())
                .and_then(|v| v.first())
                .and_then(|v| v.get("value"))
                .and_then(|v| v.as_str())
                .unwrap_or("No phone");

            results.push(format!(
                "Name: {}\nEmail: {}\nPhone: {}\n---",
                name, email, phone
            ));
        }

        if results.is_empty() {
            return Ok(ToolCallResult {
                tool_call_id: None,
                tool_name: "google_contacts_read".to_string(),
                result: "No contacts found.".to_string(),
            });
        }

        println!("\x1b[32m✅ Successfully read Google Contacts\x1b[0m");

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "google_contacts_read".to_string(),
            result: format!(
                "Found {} contacts:\n\n{}",
                results.len(),
                results.join("\n")
            ),
        })
    }

    fn is_available(&self) -> bool {
        GoogleOAuthProvider::is_configured()
    }
}
