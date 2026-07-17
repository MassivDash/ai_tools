use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::agent::tools::utility::google_oauth::GoogleOAuthProvider;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde_json::json;
use std::sync::Arc;

pub struct GoogleCalendarReadTool {
    metadata: ToolMetadata,
    oauth_provider: Arc<GoogleOAuthProvider>,
}

impl GoogleCalendarReadTool {
    pub fn new(oauth_provider: Arc<GoogleOAuthProvider>) -> Self {
        Self {
            metadata: ToolMetadata {
                id: "read_calendar_events".to_string(),
                name: "Read Calendar Events".to_string(),
                description: "Read upcoming events from the user's primary Google Calendar."
                    .to_string(),
                category: ToolCategory::Utility,
                tool_type: ToolType::GoogleCalendarRead,
            },
            oauth_provider,
        }
    }
}

#[async_trait]
impl AgentTool for GoogleCalendarReadTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "read_calendar_events",
            "description": "Read upcoming events from the user's primary Google Calendar.",
            "parameters": {
                "type": "object",
                "properties": {
                    "days_ahead": {
                        "type": "integer",
                        "description": "Number of days ahead to fetch events for. Defaults to 7."
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of events to return. Defaults to 10."
                    }
                }
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).unwrap_or_else(|_| json!({}));

        let days_ahead = args.get("days_ahead").and_then(|v| v.as_i64()).unwrap_or(7);
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_i64())
            .unwrap_or(10);

        println!(
            "\x1b[36m📅 Reading Calendar Events for next {} days (max: {})\x1b[0m",
            days_ahead, max_results
        );

        let access_token = self.oauth_provider.get_access_token().await?;

        let time_min = Utc::now();
        let time_max = time_min + Duration::days(days_ahead);

        let client = &self.oauth_provider.http_client;
        let res = client
            .get("https://www.googleapis.com/calendar/v3/calendars/primary/events")
            .bearer_auth(&access_token)
            .query(&[
                ("timeMin", time_min.to_rfc3339().as_str()),
                ("timeMax", time_max.to_rfc3339().as_str()),
                ("maxResults", max_results.to_string().as_str()),
                ("singleEvents", "true"),
                ("orderBy", "startTime"),
            ])
            .send()
            .await
            .context("Failed to request Calendar API")?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Calendar API failed with {}: {}",
                status,
                text
            ));
        }

        let data: serde_json::Value = res.json().await.unwrap_or_default();
        let items = data.get("items").and_then(|v| v.as_array());

        let items_array = match items {
            Some(arr) if !arr.is_empty() => arr,
            _ => {
                return Ok(ToolCallResult {
                    tool_name: "read_calendar_events".to_string(),
                    result: format!("No upcoming events found in the next {} days.", days_ahead),
                });
            }
        };

        let mut results = Vec::new();
        for item in items_array {
            let summary = item
                .get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled Event");
            let description = item
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Start and end times can be dateTime (for specific times) or date (for all-day events)
            let start = item
                .get("start")
                .and_then(|s| s.get("dateTime").or_else(|| s.get("date")))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown Start");

            let end = item
                .get("end")
                .and_then(|s| s.get("dateTime").or_else(|| s.get("date")))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown End");

            let html_link = item.get("htmlLink").and_then(|v| v.as_str()).unwrap_or("");

            results.push(format!(
                "---\nEvent: {}\nStart: {}\nEnd: {}\nDescription: {}\nLink: {}\n",
                summary, start, end, description, html_link
            ));
        }

        let combined_results = results.join("\n");

        println!("\x1b[32m✅ Successfully read Calendar events\x1b[0m");

        Ok(ToolCallResult {
            tool_name: "read_calendar_events".to_string(),
            result: format!(
                "Retrieved {} upcoming events:\n\n{}",
                results.len(),
                combined_results
            ),
        })
    }

    fn is_available(&self) -> bool {
        GoogleOAuthProvider::is_configured()
    }
}
