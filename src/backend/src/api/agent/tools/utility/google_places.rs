use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest;
use serde_json::json;
use std::env;

pub struct GooglePlacesSearchTool {
    metadata: ToolMetadata,
    client: reqwest::Client,
    api_key: String,
}

impl GooglePlacesSearchTool {
    pub fn new() -> Self {
        let api_key = env::var("GOOGLE_PLACES_API_KEY").unwrap_or_default();

        Self {
            metadata: ToolMetadata {
                id: "google_places_search".to_string(),
                name: "Google Places Search".to_string(),
                description:
                    "Search for places, restaurants, or businesses using Google Places API."
                        .to_string(),
                category: ToolCategory::Google,
                tool_type: ToolType::GooglePlacesSearch,
            },
            client: reqwest::Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl AgentTool for GooglePlacesSearchTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "google_places_search",
            "description": "Search for places, restaurants, businesses, or points of interest.",
            "parameters": {
                "type": "object",
                "properties": {
                    "text_query": {
                        "type": "string",
                        "description": "The text query, e.g., 'Spicy vegetarian food in Sydney'."
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Max number of results to return (between 1 and 20). Defaults to 5."
                    }
                },
                "required": ["text_query"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).unwrap_or_else(|_| json!({}));

        let text_query = args
            .get("text_query")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(5);
        let max_results = if max_results > 20 { 20 } else { max_results };

        if text_query.is_empty() {
            return Ok(ToolCallResult {
                tool_call_id: None,
                tool_name: "google_places_search".to_string(),
                result: "Error: text_query is required.".to_string(),
            });
        }

        println!(
            "\x1b[36m📍 Searching Google Places: '{}'\x1b[0m",
            text_query
        );

        let body = json!({
            "textQuery": text_query,
            "pageSize": max_results
        });

        let res = self
            .client
            .post("https://places.googleapis.com/v1/places:searchText")
            .header("X-Goog-Api-Key", &self.api_key)
            .header(
                "X-Goog-FieldMask",
                "places.id,places.displayName,places.formattedAddress,places.rating,places.userRatingCount,places.priceLevel,places.types,places.nationalPhoneNumber,places.websiteUri"
            )
            .json(&body)
            .send()
            .await
            .context("Failed to request Google Places API")?;

        if !res.status().is_success() {
            let status = res.status();
            let error_text = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Google Places API returned error {}: {}",
                status,
                error_text
            ));
        }

        let doc: serde_json::Value = res.json().await.unwrap_or_default();
        let places = doc.get("places").and_then(|v| v.as_array());

        let mut results = Vec::new();
        if let Some(arr) = places {
            for place in arr {
                let name = place
                    .get("displayName")
                    .and_then(|v| v.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Place");

                let address = place
                    .get("formattedAddress")
                    .and_then(|v| v.as_str())
                    .unwrap_or("No address");

                let rating = place.get("rating").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let ratings_count = place
                    .get("userRatingCount")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let price_level = place
                    .get("priceLevel")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                let mut types_str = String::new();
                if let Some(types) = place.get("types").and_then(|v| v.as_array()) {
                    let types_list: Vec<String> = types
                        .iter()
                        .filter_map(|t| t.as_str().map(String::from))
                        .collect();
                    types_str = types_list.join(", ");
                }

                let phone = place
                    .get("nationalPhoneNumber")
                    .and_then(|v| v.as_str())
                    .unwrap_or("No phone");
                let website = place
                    .get("websiteUri")
                    .and_then(|v| v.as_str())
                    .unwrap_or("No website");

                results.push(format!(
                    "Name: {}\nAddress: {}\nRating: {} ({} reviews)\nPrice: {}\nTypes: {}\nPhone: {}\nWebsite: {}\n---",
                    name, address, rating, ratings_count, price_level, types_str, phone, website
                ));
            }
        }

        if results.is_empty() {
            return Ok(ToolCallResult {
                tool_call_id: None,
                tool_name: "google_places_search".to_string(),
                result: "No places found.".to_string(),
            });
        }

        println!("\x1b[32m✅ Successfully searched Google Places\x1b[0m");

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "google_places_search".to_string(),
            result: format!("Found {} places:\n\n{}", results.len(), results.join("\n")),
        })
    }

    fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }
}
