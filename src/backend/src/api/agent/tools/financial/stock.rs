use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest;
use serde_json::json;
use std::env;

/// Stock tool for fetching stock market data from Alpha Vantage API
pub struct StockTool {
    metadata: ToolMetadata,
    client: reqwest::Client,
    api_key: Option<String>,
}

impl StockTool {
    /// Create a new instance of the stock tool
    pub fn new() -> Self {
        let api_key = env::var("ALPHA_ADVANTAGE_KEY").ok();

        Self {
            metadata: ToolMetadata {
                id: "6".to_string(),
                name: "Stock Market Data".to_string(),
                description: "Fetch stock market data (daily/weekly/monthly time series) via Alpha Vantage API".to_string(),
                category: ToolCategory::Financial,
                tool_type: ToolType::Stock,
            },
            client: reqwest::Client::new(),
            api_key,
        }
    }

    /// Fetch stock data from Alpha Vantage API
    async fn fetch_stock_data(
        &self,
        function: &str,
        symbol: &str,
        outputsize: Option<&str>,
    ) -> Result<serde_json::Value> {
        let api_key = self
            .api_key
            .as_ref()
            .context("ALPHA_ADVANTAGE_KEY environment variable not set")?;

        let base_url = "https://www.alphavantage.co/query";

        let mut url = format!(
            "{}?function={}&symbol={}&apikey={}",
            base_url, function, symbol, api_key
        );

        if let Some(size) = outputsize {
            url.push_str(&format!("&outputsize={}", size));
        }

        println!(
            "\x1b[33m📈 Fetching stock data from Alpha Vantage: {}\x1b[0m",
            url.replace(api_key, "***")
        );

        let response = self
            .client
            .get(&url)
            .header("User-Agent", "ai_tools/1.0")
            .send()
            .await
            .context("Failed to request stock data from Alpha Vantage API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Alpha Vantage API returned error {}: {}",
                status,
                error_text
            ));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Alpha Vantage API response")?;

        // Check for API error messages
        if let Some(error_msg) = data.get("Error Message") {
            return Err(anyhow::anyhow!(
                "Alpha Vantage API error: {}",
                error_msg.as_str().unwrap_or("Unknown error")
            ));
        }

        if let Some(note) = data.get("Note") {
            return Err(anyhow::anyhow!(
                "Alpha Vantage API limit reached: {}",
                note.as_str().unwrap_or("Rate limit exceeded")
            ));
        }

        Ok(data)
    }

    fn format_stock_response(
        &self,
        data: &serde_json::Value,
        function: &str,
        limit: Option<usize>,
    ) -> Result<String> {
        let mut result = String::new();

        // Get metadata
        if let Some(meta) = data.get("Meta Data") {
            let symbol = meta
                .get("2. Symbol")
                .and_then(|s| s.as_str())
                .unwrap_or("Unknown");
            let last_refreshed = meta
                .get("3. Last Refreshed")
                .and_then(|s| s.as_str())
                .unwrap_or("Unknown");

            result.push_str(&format!("📊 **Stock Data for {}**\n", symbol));
            result.push_str(&format!("🕐 Last Refreshed: {}\n\n", last_refreshed));
        }

        // Determine which time series key to use
        let time_series_key = match function {
            "TIME_SERIES_DAILY" => "Time Series (Daily)",
            "TIME_SERIES_WEEKLY" => "Weekly Time Series",
            "TIME_SERIES_MONTHLY" => "Monthly Time Series",
            _ => "Time Series (Daily)",
        };

        if let Some(time_series) = data.get(time_series_key).and_then(|ts| ts.as_object()) {
            // Get the most recent entries
            let mut entries: Vec<_> = time_series.iter().collect();
            entries.sort_by(|a, b| b.0.cmp(a.0)); // Sort by date descending

            // Use provided limit or default to 10 if not specified
            // If limit is 0, show all (careful!)
            let limit_val = limit.unwrap_or(10);
            let display_count = if limit_val == 0 {
                entries.len()
            } else {
                entries.len().min(limit_val)
            };

            // Take the recent ones, but then reverse them to show in chronological order (Oldest -> Newest)
            let recent_entries: Vec<_> = entries.iter().take(display_count).rev().collect();

            result.push_str(&format!(
                "📅 **Recent {} entries (Chronological):**\n\n",
                display_count
            ));

            for (date, values) in recent_entries {
                let open = values
                    .get("1. open")
                    .and_then(|v| v.as_str())
                    .unwrap_or("N/A");
                let high = values
                    .get("2. high")
                    .and_then(|v| v.as_str())
                    .unwrap_or("N/A");
                let low = values
                    .get("3. low")
                    .and_then(|v| v.as_str())
                    .unwrap_or("N/A");
                let close = values
                    .get("4. close")
                    .and_then(|v| v.as_str())
                    .unwrap_or("N/A");
                let volume = values
                    .get("5. volume")
                    .and_then(|v| v.as_str())
                    .unwrap_or("N/A");

                result.push_str(&format!(
                    "**{}**\n  • Open: ${} | High: ${} | Low: ${} | Close: ${}\n  • Volume: {}\n\n",
                    date, open, high, low, close, volume
                ));
            }

            if entries.len() > display_count {
                result.push_str(&format!(
                    "_...and {} more entries available_\n",
                    entries.len() - display_count
                ));
            }

            // Add instruction for charts if the user might be interested
            result.push_str("\n💡 **To display a chart:**\n");
            result.push_str("If the user asked for a chart, output the data in a `json-chart` code block strictly following this schema:\n");
            result.push_str("```json-chart\n");
            result.push_str("{\n");
            result.push_str("  \"type\": \"line\", // or \"bar\"\n");
            result.push_str("  \"title\": \"Stock Price History\",\n");
            result.push_str("  \"xAxis\": { \"label\": \"Date\", \"data\": [\"2023-01-01\", \"2023-01-02\"] },\n");
            result.push_str("  \"series\": [\n");
            result.push_str("    { \"name\": \"Close Price\", \"data\": [150.5, 152.3] }\n");
            result.push_str("  ]\n");
            result.push_str("}\n");
            result.push_str("```\n");
        } else {
            result.push_str("No time series data found in response.\n");
        }

        if result.is_empty() {
            Ok("No stock data found or format not recognized.".to_string())
        } else {
            Ok(result)
        }
    }
}

#[async_trait]
impl AgentTool for StockTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "stock_data",
            "description": "Fetch stock market data (OHLCV) for a given stock symbol. CHOOSE THE BEST FUNCTION based on the time range requested:\n- **TIME_SERIES_DAILY**: Use for recent data (last few days, last week, up to 2 months).\n- **TIME_SERIES_WEEKLY**: Use for medium-term data (last 2 months to 2 years).\n- **TIME_SERIES_MONTHLY**: Use for long-term data (over 2 years).\n\nExamples:\n- 'last 7 days': TIME_SERIES_DAILY\n- 'last 10 weeks': TIME_SERIES_WEEKLY\n- 'last 5 years': TIME_SERIES_MONTHLY\n\nWhen chart is requested, use this data to generate a json-chart.",
            "parameters": {
                "type": "object",
                "properties": {
                    "symbol": {
                        "type": "string",
                        "description": "Stock ticker symbol (e.g., 'NVDA' for Nvidia, 'AAPL' for Apple)."
                    },
                    "function": {
                        "type": "string",
                        "description": "Time series function: 'TIME_SERIES_DAILY', 'TIME_SERIES_WEEKLY', 'TIME_SERIES_MONTHLY'.",
                        "enum": ["TIME_SERIES_DAILY", "TIME_SERIES_WEEKLY", "TIME_SERIES_MONTHLY"],
                        "default": "TIME_SERIES_DAILY"
                    },
                    "outputsize": {
                        "type": "string",
                        "description": "Output size: 'compact' (latest 100) or 'full'.",
                        "enum": ["compact", "full"],
                        "default": "compact"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Limit the number of results returned (i.e. 'last 5 days' = 5). Default is 10. Use 0 for all available.",
                        "default": 10
                    }
                },
                "required": ["symbol"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse stock tool arguments")?;

        let symbol = args
            .get("symbol")
            .and_then(|v| v.as_str())
            .context("Missing required 'symbol' parameter")?
            .to_uppercase();

        let function = args
            .get("function")
            .and_then(|v| v.as_str())
            .unwrap_or("TIME_SERIES_DAILY");

        let outputsize = args.get("outputsize").and_then(|v| v.as_str());
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        // Validate function parameter
        let valid_functions = [
            "TIME_SERIES_DAILY",
            "TIME_SERIES_WEEKLY",
            "TIME_SERIES_MONTHLY",
        ];
        if !valid_functions.contains(&function) {
            return Err(anyhow::anyhow!(
                "Invalid function '{}'. Must be one of: TIME_SERIES_DAILY, TIME_SERIES_WEEKLY, TIME_SERIES_MONTHLY",
                function
            ));
        }

        let data = self.fetch_stock_data(function, &symbol, outputsize).await?;
        let result = self.format_stock_response(&data, function, limit)?;

        Ok(ToolCallResult {
            tool_name: "stock_data".to_string(),
            result,
        })
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }
}
