use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use crate::api::agent::tools::utility::google_oauth::GoogleOAuthProvider;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct GoogleCalendarTool {
    metadata: ToolMetadata,
    oauth_provider: Arc<GoogleOAuthProvider>,
}

impl GoogleCalendarTool {
    pub fn new(oauth_provider: Arc<GoogleOAuthProvider>) -> Self {
        Self {
            metadata: ToolMetadata {
                id: "create_calendar_event".to_string(),
                name: "Create Calendar Event".to_string(),
                description: "Create an event on the user's primary Google Calendar.".to_string(),
                category: ToolCategory::Google,
                tool_type: ToolType::GoogleCalendar,
            },
            oauth_provider,
        }
    }
}

#[async_trait]
impl AgentTool for GoogleCalendarTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "create_calendar_event",
            "description": "Create an event on the user's primary Google Calendar.",
            "parameters": {
                "type": "object",
                "properties": {
                    "summary": {
                        "type": "string",
                        "description": "The title or summary of the event."
                    },
                    "description": {
                        "type": "string",
                        "description": "A description of the event."
                    },
                    "start_time": {
                        "type": "string",
                        "description": "The start time of the event in ISO 8601 format (e.g., 2026-07-17T15:00:00Z)."
                    },
                    "end_time": {
                        "type": "string",
                        "description": "The end time of the event in ISO 8601 format. If omitted, defaults to 1 hour after start_time."
                    },
                    "timezone": {
                        "type": "string",
                        "description": "The IANA timezone ID (e.g., 'America/New_York', 'Europe/Berlin'). CRITICAL for recurring events to handle Daylight Saving Time correctly. If omitted, Google will use UTC."
                    },
                    "recurrence": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional recurrence rules (e.g., [\"RRULE:FREQ=WEEKLY;COUNT=10\"])."
                    }
                },
                "required": ["summary", "start_time"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse arguments")?;

        let summary = args
            .get("summary")
            .and_then(|v| v.as_str())
            .context("Missing 'summary'")?;
        let description = args
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let start_time_str = args
            .get("start_time")
            .and_then(|v| v.as_str())
            .context("Missing 'start_time'")?;

        // Parse start_time to handle default end_time
        let start_time = chrono::DateTime::parse_from_rfc3339(start_time_str)
            .or_else(|_| chrono::DateTime::parse_from_rfc3339(&format!("{}Z", start_time_str)))
            .context("Invalid 'start_time' format. Must be ISO 8601")?;

        let recurrence = args.get("recurrence").and_then(|v| v.as_array()).cloned();
        let timezone = args.get("timezone").and_then(|v| v.as_str());

        let end_time_str = if let Some(end_str) = args.get("end_time").and_then(|v| v.as_str()) {
            end_str.to_string()
        } else {
            // Default to 1 hour
            let end_time = start_time + chrono::Duration::hours(1);
            end_time.to_rfc3339()
        };

        println!(
            "\x1b[36m📅 Creating Calendar Event: {} at {}\x1b[0m",
            summary, start_time_str
        );

        // Fetch access token
        let access_token = self.oauth_provider.get_access_token().await?;

        let mut start_obj = serde_json::Map::new();
        start_obj.insert("dateTime".to_string(), json!(start_time_str));

        let mut end_obj = serde_json::Map::new();
        end_obj.insert("dateTime".to_string(), json!(end_time_str));

        if let Some(tz) = timezone {
            start_obj.insert("timeZone".to_string(), json!(tz));
            end_obj.insert("timeZone".to_string(), json!(tz));
        }

        let mut payload = serde_json::Map::new();
        payload.insert("summary".to_string(), json!(summary));
        payload.insert("description".to_string(), json!(description));
        payload.insert("start".to_string(), serde_json::Value::Object(start_obj));
        payload.insert("end".to_string(), serde_json::Value::Object(end_obj));

        if let Some(r) = recurrence {
            payload.insert("recurrence".to_string(), serde_json::Value::Array(r));
        }

        let res = self
            .oauth_provider
            .http_client
            .post("https://www.googleapis.com/calendar/v3/calendars/primary/events")
            .bearer_auth(access_token)
            .json(&payload)
            .send()
            .await
            .context("Failed to send request to Calendar API")?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Calendar API failed with {}: {}",
                status,
                text
            ));
        }

        let event_data: serde_json::Value = res.json().await.unwrap_or_default();
        let html_link = event_data
            .get("htmlLink")
            .and_then(|v| v.as_str())
            .unwrap_or("No link provided");

        println!("\x1b[32m✅ Calendar event created successfully\x1b[0m");

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "create_calendar_event".to_string(),
            result: format!(
                "Successfully created event '{}'. Link: {}",
                summary, html_link
            ),
        })
    }

    fn is_available(&self) -> bool {
        GoogleOAuthProvider::is_configured()
    }
}
