use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest;
use serde_json::json;
use std::env;

/// Crypto tool for fetching exchange rates and crypto history from Alpha Vantage API
pub struct CryptoTool {
    metadata: ToolMetadata,
    client: reqwest::Client,
    api_key: Option<String>,
}

impl CryptoTool {
    /// Create a new instance of the crypto tool
    pub fn new() -> Self {
        let api_key = env::var("ALPHA_ADVANTAGE_KEY").ok();

        Self {
            metadata: ToolMetadata {
                id: "7".to_string(),
                name: "Cryptocurrency & Global Exchange".to_string(),
                description: "Fetch real-time cryptocurrency exchange rates and historical crypto data (daily/weekly/monthly) via Alpha Vantage API".to_string(),
                category: ToolCategory::Financial,
                tool_type: ToolType::Crypto,
            },
            client: reqwest::Client::new(),
            api_key,
        }
    }

    /// Fetch data from Alpha Vantage API
    async fn fetch_data(
        &self,
        function: &str,
        from_currency: &str,
        to_currency: &str,
    ) -> Result<serde_json::Value> {
        let api_key = self
            .api_key
            .as_ref()
            .context("ALPHA_ADVANTAGE_KEY environment variable not set")?;

        let base_url = "https://www.alphavantage.co/query";
        let mut url = format!("{}?function={}&apikey={}", base_url, function, api_key);

        if function == "CURRENCY_EXCHANGE_RATE" {
            url.push_str(&format!(
                "&from_currency={}&to_currency={}",
                from_currency, to_currency
            ));
        } else {
            // For DIGITAL_CURRENCY_*, parameters are 'symbol' and 'market'
            // Mapping: from_currency -> symbol, to_currency -> market
            url.push_str(&format!("&symbol={}&market={}", from_currency, to_currency));
        }

        println!(
            "\x1b[33m🪙 Fetching crypto data from Alpha Vantage: {}\x1b[0m",
            url.replace(api_key, "***")
        );

        let response = self
            .client
            .get(&url)
            .header("User-Agent", "ai_tools/1.0")
            .send()
            .await
            .context("Failed to request data from Alpha Vantage API")?;

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
            // Sometimes it's just a warning, but often it means rate limit
            if note.as_str().unwrap_or("").contains("Thank you") {
                // Rate limit
                return Err(anyhow::anyhow!(
                    "Alpha Vantage API limit reached: {}",
                    note.as_str().unwrap_or("Rate limit exceeded")
                ));
            }
        }

        Ok(data)
    }

    fn format_response(
        &self,
        data: &serde_json::Value,
        function: &str,
        from_symbol: &str,
        to_symbol: &str,
    ) -> Result<String> {
        let mut result = String::new();

        if function == "CURRENCY_EXCHANGE_RATE" {
            // Expected format: { "Realtime Currency Exchange Rate": { "1. From_Currency Code": "BTC", ... } }
            if let Some(rate_data) = data
                .get("Realtime Currency Exchange Rate")
                .and_then(|v| v.as_object())
            {
                let from_code = rate_data
                    .get("1. From_Currency Code")
                    .and_then(|s| s.as_str())
                    .unwrap_or(from_symbol);
                let from_name = rate_data
                    .get("2. From_Currency Name")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                let to_code = rate_data
                    .get("3. To_Currency Code")
                    .and_then(|s| s.as_str())
                    .unwrap_or(to_symbol);
                let to_name = rate_data
                    .get("4. To_Currency Name")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                let rate = rate_data
                    .get("5. Exchange Rate")
                    .and_then(|s| s.as_str())
                    .unwrap_or("N/A");
                let last_refreshed = rate_data
                    .get("6. Last Refreshed")
                    .and_then(|s| s.as_str())
                    .unwrap_or("N/A");
                let bid = rate_data
                    .get("8. Bid Price")
                    .and_then(|s| s.as_str())
                    .unwrap_or("N/A");
                let ask = rate_data
                    .get("9. Ask Price")
                    .and_then(|s| s.as_str())
                    .unwrap_or("N/A");

                result.push_str(&format!(
                    "💱 **Exchange Rate: {} ({}) to {} ({})**\n\n",
                    from_code, from_name, to_code, to_name
                ));
                result.push_str(&format!("💰 **Rate: {}**\n", rate));
                result.push_str(&format!("🕒 Last Refreshed: {}\n", last_refreshed));
                result.push_str(&format!("• Bid: {}\n", bid));
                result.push_str(&format!("• Ask: {}\n", ask));
            } else {
                result.push_str("No exchange rate data found in response.\n");
            }
        } else {
            // DIGITAL_CURRENCY_DAILY, DIGITAL_CURRENCY_WEEKLY or DIGITAL_CURRENCY_MONTHLY
            let meta_key = "Meta Data";
            if let Some(meta) = data.get(meta_key) {
                let symbol = meta
                    .get("2. Digital Currency Code")
                    .and_then(|s| s.as_str())
                    .unwrap_or(from_symbol);
                let name = meta
                    .get("3. Digital Currency Name")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                let market = meta
                    .get("4. Market Code")
                    .and_then(|s| s.as_str())
                    .unwrap_or(to_symbol);
                let last_refreshed = meta
                    .get("6. Last Refreshed")
                    .and_then(|s| s.as_str())
                    .unwrap_or("Unknown");

                result.push_str(&format!(
                    "📊 **Crypto Data for {} ({}) in {}**\n",
                    symbol, name, market
                ));
                result.push_str(&format!("🕐 Last Refreshed: {}\n\n", last_refreshed));
            }

            let series_key = match function {
                "DIGITAL_CURRENCY_DAILY" => "Time Series (Digital Currency Daily)",
                "DIGITAL_CURRENCY_WEEKLY" => "Time Series (Digital Currency Weekly)",
                "DIGITAL_CURRENCY_MONTHLY" => "Time Series (Digital Currency Monthly)",
                _ => "Time Series (Digital Currency Daily)", // Fallback
            };

            if let Some(time_series) = data.get(series_key).and_then(|ts| ts.as_object()) {
                let mut entries: Vec<_> = time_series.iter().collect();
                entries.sort_by(|a, b| b.0.cmp(a.0)); // Sort by date descending to get most recent
                let display_count = entries.len().min(10); // Show recent 10

                // Take the recent ones, but then reverse them to show in chronological order (Oldest -> Newest)
                let recent_entries: Vec<_> = entries.iter().take(display_count).rev().collect();

                result.push_str(&format!(
                    "📅 **Recent {} entries (Chronological):**\n\n",
                    display_count
                ));

                for (date, values) in recent_entries {
                    // Try to get open/close/volume with requested market currency
                    // Fallback to USD if specific market currency keys are not found (Alpha Vantage often provides USD)
                    // Also fallback to simple "1. open" style keys seen in some responses

                    let open_key = format!("1a. open ({})", to_symbol);
                    let open_usd_key = "1a. open (USD)";
                    let open_simple_key = "1. open";

                    let open = values
                        .get(&open_key)
                        .or_else(|| values.get(open_usd_key))
                        .or_else(|| values.get(open_simple_key))
                        .and_then(|v| v.as_str())
                        .unwrap_or("N/A");

                    let close_key = format!("4a. close ({})", to_symbol);
                    let close_usd_key = "4a. close (USD)";
                    let close_simple_key = "4. close";

                    let close = values
                        .get(&close_key)
                        .or_else(|| values.get(close_usd_key))
                        .or_else(|| values.get(close_simple_key))
                        .and_then(|v| v.as_str())
                        .unwrap_or("N/A");

                    let volume = values
                        .get("5. volume")
                        .and_then(|v| v.as_str())
                        .unwrap_or("N/A");

                    result.push_str(&format!(
                        "**{}**\n  • Open: {} {}\n  • Close: {} {}\n  • Volume: {}\n\n",
                        date, open, to_symbol, close, to_symbol, volume
                    ));
                }

                // Add instruction for charts
                result.push_str("\n💡 **To display a chart:**\n");
                result.push_str("If the user asked for a chart, output the data in a `json-chart` code block strictly following this schema:\n");
                result.push_str("```json-chart\n");
                result.push_str("{\n");
                result.push_str("  \"type\": \"line\",\n");
                result.push_str("  \"title\": \"Crypto Price History\",\n");
                result.push_str("  \"xAxis\": { \"label\": \"Date\", \"data\": [...] },\n");
                result.push_str("  \"series\": [\n");
                result.push_str("    { \"name\": \"Close Price\", \"data\": [...] }\n");
                result.push_str("  ]\n");
                result.push_str("}\n");
                result.push_str("```\n");
            } else {
                result.push_str("No time series data found.\n");
            }
        }

        if result.is_empty() {
            Ok("No data found or format not recognized.".to_string())
        } else {
            Ok(result)
        }
    }
}

#[async_trait]
impl AgentTool for CryptoTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "crypto_data",
            "description": "Fetch real-time cryptocurrency exchange rates or historical cryptocurrency data. Use 'CURRENCY_EXCHANGE_RATE' for any currency pair (fiat/crypto).\n\nFor historical data, CHOOSE THE BEST FUNCTION based on the time range requested:\n- **DIGITAL_CURRENCY_DAILY**: Use for recent data (last few days, last week, up to 2 months).\n- **DIGITAL_CURRENCY_WEEKLY**: Use for medium-term data (last 2 months to 2 years).\n- **DIGITAL_CURRENCY_MONTHLY**: Use for long-term data (over 2 years).\n\nExamples:\n- 'last 7 days': DIGITAL_CURRENCY_DAILY\n- 'last 10 weeks': DIGITAL_CURRENCY_WEEKLY\n- 'last 5 years': DIGITAL_CURRENCY_MONTHLY",
            "parameters": {
                "type": "object",
                "properties": {
                    "function": {
                        "type": "string",
                        "description": "Function to perform: 'CURRENCY_EXCHANGE_RATE', 'DIGITAL_CURRENCY_DAILY', 'DIGITAL_CURRENCY_WEEKLY', 'DIGITAL_CURRENCY_MONTHLY'.",
                        "enum": ["CURRENCY_EXCHANGE_RATE", "DIGITAL_CURRENCY_DAILY", "DIGITAL_CURRENCY_WEEKLY", "DIGITAL_CURRENCY_MONTHLY"],
                        "default": "CURRENCY_EXCHANGE_RATE"
                    },
                    "from_currency": {
                        "type": "string",
                        "description": "Base currency code (e.g. 'USD', 'EUR', 'BTC'). For crypto history, this is the cryptocurrency symbol."
                    },
                    "to_currency": {
                        "type": "string",
                        "description": "Target currency code (e.g. 'JPY', 'CNY', 'USD'). For crypto history, this is the market currency."
                    }
                },
                "required": ["from_currency", "to_currency"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse crypto tool arguments")?;

        let function = args
            .get("function")
            .and_then(|v| v.as_str())
            .unwrap_or("CURRENCY_EXCHANGE_RATE");

        let from_currency = args
            .get("from_currency")
            .and_then(|v| v.as_str())
            .context("Missing required 'from_currency' parameter")?
            .to_uppercase();

        let to_currency = args
            .get("to_currency")
            .and_then(|v| v.as_str())
            .context("Missing required 'to_currency' parameter")?
            .to_uppercase();

        println!(
            "🪙 CryptoTool executing: function={}, from={}, to={}",
            function, from_currency, to_currency
        );

        // Validate function
        let valid_functions = [
            "CURRENCY_EXCHANGE_RATE",
            "DIGITAL_CURRENCY_DAILY",
            "DIGITAL_CURRENCY_WEEKLY",
            "DIGITAL_CURRENCY_MONTHLY",
        ];
        if !valid_functions.contains(&function) {
            return Err(anyhow::anyhow!(
                "Invalid function '{}'. Must be one of: {:?}",
                function,
                valid_functions
            ));
        }

        let data = self
            .fetch_data(function, &from_currency, &to_currency)
            .await?;
        let result = self.format_response(&data, function, &from_currency, &to_currency)?;

        Ok(ToolCallResult {
            tool_name: "crypto_data".to_string(),
            result,
        })
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }
}
