use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest;
use serde_json::json;
use std::env;

/// The real Alpha Vantage query endpoint, used unless a test overrides it.
const ALPHA_VANTAGE_URL: &str = "https://www.alphavantage.co/query";

/// Crypto tool for fetching exchange rates and crypto history from Alpha Vantage API
pub struct CryptoTool {
    metadata: ToolMetadata,
    client: reqwest::Client,
    api_key: Option<String>,
    /// Query endpoint to talk to. Always the real Alpha Vantage one in
    /// production; tests point it at a loopback mock instead.
    base_url: String,
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
            base_url: ALPHA_VANTAGE_URL.to_string(),
        }
    }

    /// A tool with a canned API key pointed at `base_url` instead of the real
    /// Alpha Vantage endpoint, so the request/response handling can be driven
    /// without either the network or the `ALPHA_ADVANTAGE_KEY` env var.
    #[cfg(test)]
    pub(crate) fn with_base_url(base_url: impl Into<String>, api_key: Option<&str>) -> Self {
        Self {
            api_key: api_key.map(|key| key.to_string()),
            base_url: base_url.into(),
            ..Self::new()
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

        let base_url = &self.base_url;
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
        limit: Option<usize>,
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

                    let high_key = format!("2a. high ({})", to_symbol);
                    let high_usd_key = "2a. high (USD)";
                    let high_simple_key = "2. high";

                    let high = values
                        .get(&high_key)
                        .or_else(|| values.get(high_usd_key))
                        .or_else(|| values.get(high_simple_key))
                        .and_then(|v| v.as_str())
                        .unwrap_or("N/A");

                    let low_key = format!("3a. low ({})", to_symbol);
                    let low_usd_key = "3a. low (USD)";
                    let low_simple_key = "3. low";

                    let low = values
                        .get(&low_key)
                        .or_else(|| values.get(low_usd_key))
                        .or_else(|| values.get(low_simple_key))
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
                        "**{}**\n  • Open: {} {} | High: {} | Low: {} | Close: {} | Vol: {}\n\n",
                        date, open, to_symbol, high, low, close, volume
                    ));
                }

                if entries.len() > display_count {
                    result.push_str(&format!(
                        "_...and {} more entries available_\n",
                        entries.len() - display_count
                    ));
                }
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
            "description": "Fetch real-time cryptocurrency exchange rates or historical cryptocurrency data. Use 'CURRENCY_EXCHANGE_RATE' for any currency pair (fiat/crypto).\n\nFor historical data, CHOOSE THE BEST FUNCTION based on the time range requested:\n- **DIGITAL_CURRENCY_DAILY**: Use for recent data (last few days, last week, up to 2 months).\n- **DIGITAL_CURRENCY_WEEKLY**: Use for medium-term data (last 2 months to 2 years).\n- **DIGITAL_CURRENCY_MONTHLY**: Use for long-term data (over 2 years).\n\nExamples:\n- 'last 7 days': DIGITAL_CURRENCY_DAILY\n- 'last 10 weeks': DIGITAL_CURRENCY_WEEKLY\n- 'last 5 years': DIGITAL_CURRENCY_MONTHLY\n\nWhen chart is requested, use this data to generate a json-chart.",
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
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Limit the number of results returned (i.e. 'last 5 days' = 5). Default is 10. Use 0 for all available.",
                        "default": 10
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

        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

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
        let result = self.format_response(&data, function, &from_currency, &to_currency, limit)?;

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "crypto_data".to_string(),
            result,
        })
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::core::types::FunctionCall;
    use crate::test_support::{MockHttpApi, MockResponse};

    /// Alpha Vantage's single endpoint, relative to the mock's root. Using the
    /// real path shape means the recorded request is directly comparable to what
    /// the live API would have received.
    const QUERY_PATH: &str = "/query";

    fn tool_call(arguments: &str) -> ToolCall {
        ToolCall {
            id: "call_crypto".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "crypto_data".to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    /// A tool talking to `api`, with a canned key so no env var is involved.
    fn tool_for(api: &MockHttpApi) -> CryptoTool {
        CryptoTool::with_base_url(api.url(QUERY_PATH), Some("test-key"))
    }

    fn exchange_rate_body() -> serde_json::Value {
        json!({
            "Realtime Currency Exchange Rate": {
                "1. From_Currency Code": "BTC",
                "2. From_Currency Name": "Bitcoin",
                "3. To_Currency Code": "USD",
                "4. To_Currency Name": "United States Dollar",
                "5. Exchange Rate": "64000.10000000",
                "6. Last Refreshed": "2026-08-03 12:00:00",
                "8. Bid Price": "63999.00000000",
                "9. Ask Price": "64001.00000000"
            }
        })
    }

    #[test]
    fn metadata_and_function_definition_describe_the_crypto_tool() {
        let tool = CryptoTool::new();
        assert_eq!(tool.metadata().id, "7");
        assert_eq!(tool.metadata().category, ToolCategory::Financial);
        assert_eq!(tool.metadata().tool_type, ToolType::Crypto);

        let def = tool.get_function_definition();
        assert_eq!(def["name"], "crypto_data");
        assert_eq!(
            def["parameters"]["required"],
            json!(["from_currency", "to_currency"])
        );
        assert_eq!(
            def["parameters"]["properties"]["function"]["default"],
            "CURRENCY_EXCHANGE_RATE"
        );
    }

    #[test]
    fn availability_follows_the_api_key() {
        assert!(CryptoTool::with_base_url("http://unused", Some("k")).is_available());
        assert!(!CryptoTool::with_base_url("http://unused", None).is_available());
    }

    #[tokio::test]
    async fn exchange_rate_sends_currency_pair_and_formats_the_rate() {
        let api =
            MockHttpApi::serving("GET", QUERY_PATH, MockResponse::json(exchange_rate_body())).await;

        let result = tool_for(&api)
            .execute(&tool_call(
                r#"{"from_currency": "btc", "to_currency": "usd"}"#,
            ))
            .await
            .expect("The exchange rate call should succeed");

        // Exactly the query Alpha Vantage's CURRENCY_EXCHANGE_RATE expects, with
        // both codes upper-cased by execute().
        let request = api.only_request();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, QUERY_PATH);
        assert_eq!(
            request.query_params(),
            vec![
                ("function".to_string(), "CURRENCY_EXCHANGE_RATE".to_string()),
                ("apikey".to_string(), "test-key".to_string()),
                ("from_currency".to_string(), "BTC".to_string()),
                ("to_currency".to_string(), "USD".to_string()),
            ]
        );
        assert_eq!(request.header("user-agent"), Some("ai_tools/1.0"));

        assert_eq!(result.tool_name, "crypto_data");
        assert!(result.tool_call_id.is_none());
        assert!(result
            .result
            .contains("Exchange Rate: BTC (Bitcoin) to USD (United States Dollar)"));
        assert!(result.result.contains("Rate: 64000.10000000"));
        assert!(result
            .result
            .contains("Last Refreshed: 2026-08-03 12:00:00"));
        assert!(result.result.contains("Bid: 63999.00000000"));
        assert!(result.result.contains("Ask: 64001.00000000"));

        api.stop().await;
    }

    #[tokio::test]
    async fn exchange_rate_with_unrecognised_body_says_so() {
        let api = MockHttpApi::serving("GET", QUERY_PATH, MockResponse::json(json!({}))).await;

        let result = tool_for(&api)
            .execute(&tool_call(
                r#"{"from_currency": "BTC", "to_currency": "USD"}"#,
            ))
            .await
            .expect("An empty body is not an error, just unformattable");

        assert_eq!(result.result, "No exchange rate data found in response.\n");
        api.stop().await;
    }

    #[tokio::test]
    async fn daily_series_sends_symbol_and_market_and_honours_the_limit() {
        let body = json!({
            "Meta Data": {
                "2. Digital Currency Code": "BTC",
                "3. Digital Currency Name": "Bitcoin",
                "4. Market Code": "EUR",
                "6. Last Refreshed": "2026-08-03 00:00:00"
            },
            "Time Series (Digital Currency Daily)": {
                "2026-08-01": {"1a. open (EUR)": "1.0", "2a. high (EUR)": "1.5", "3a. low (EUR)": "0.5", "4a. close (EUR)": "1.2", "5. volume": "100"},
                "2026-08-02": {"1a. open (EUR)": "2.0", "2a. high (EUR)": "2.5", "3a. low (EUR)": "1.5", "4a. close (EUR)": "2.2", "5. volume": "200"},
                "2026-08-03": {"1a. open (EUR)": "3.0", "2a. high (EUR)": "3.5", "3a. low (EUR)": "2.5", "4a. close (EUR)": "3.2", "5. volume": "300"}
            }
        });
        let api = MockHttpApi::serving("GET", QUERY_PATH, MockResponse::json(body)).await;

        let result = tool_for(&api)
            .execute(&tool_call(
                r#"{"function": "DIGITAL_CURRENCY_DAILY", "from_currency": "btc", "to_currency": "eur", "limit": 2}"#,
            ))
            .await
            .expect("The daily series call should succeed");

        // Historical functions use symbol/market, not from_currency/to_currency.
        let request = api.only_request();
        assert_eq!(
            request.query_params(),
            vec![
                ("function".to_string(), "DIGITAL_CURRENCY_DAILY".to_string()),
                ("apikey".to_string(), "test-key".to_string()),
                ("symbol".to_string(), "BTC".to_string()),
                ("market".to_string(), "EUR".to_string()),
            ]
        );

        assert!(result
            .result
            .contains("Crypto Data for BTC (Bitcoin) in EUR"));
        assert!(result.result.contains("Recent 2 entries (Chronological):"));
        // The two most recent entries, oldest first, and the third dropped.
        let second = result
            .result
            .find("**2026-08-02**")
            .expect("2026-08-02 shown");
        let third = result
            .result
            .find("**2026-08-03**")
            .expect("2026-08-03 shown");
        assert!(
            second < third,
            "Entries must be chronological: {}",
            result.result
        );
        assert!(!result.result.contains("2026-08-01"));
        assert!(result.result.contains("_...and 1 more entries available_"));
        assert!(result
            .result
            .contains("Open: 2.0 EUR | High: 2.5 | Low: 1.5 | Close: 2.2 | Vol: 200"));

        api.stop().await;
    }

    #[tokio::test]
    async fn series_falls_back_to_usd_then_plain_keys_when_market_keys_are_absent() {
        // Alpha Vantage has shipped all three of these shapes; the formatter is
        // meant to degrade from "(MARKET)" to "(USD)" to bare "1. open".
        let body = json!({
            "Time Series (Digital Currency Weekly)": {
                "2026-07-27": {"1a. open (USD)": "9.0", "2a. high (USD)": "9.5", "3a. low (USD)": "8.5", "4a. close (USD)": "9.2"},
                "2026-08-03": {"1. open": "7.0", "2. high": "7.5", "3. low": "6.5", "4. close": "7.2"}
            }
        });
        let api = MockHttpApi::serving("GET", QUERY_PATH, MockResponse::json(body)).await;

        let result = tool_for(&api)
            .execute(&tool_call(
                r#"{"function": "DIGITAL_CURRENCY_WEEKLY", "from_currency": "BTC", "to_currency": "PLN"}"#,
            ))
            .await
            .expect("The weekly series call should succeed");

        assert!(result
            .result
            .contains("Open: 9.0 PLN | High: 9.5 | Low: 8.5 | Close: 9.2 | Vol: N/A"));
        assert!(result
            .result
            .contains("Open: 7.0 PLN | High: 7.5 | Low: 6.5 | Close: 7.2 | Vol: N/A"));
        api.stop().await;
    }

    #[tokio::test]
    async fn limit_zero_shows_every_entry() {
        let body = json!({
            "Time Series (Digital Currency Monthly)": {
                "2026-06-30": {"1. open": "1.0"},
                "2026-07-31": {"1. open": "2.0"}
            }
        });
        let api = MockHttpApi::serving("GET", QUERY_PATH, MockResponse::json(body)).await;

        let result = tool_for(&api)
            .execute(&tool_call(
                r#"{"function": "DIGITAL_CURRENCY_MONTHLY", "from_currency": "BTC", "to_currency": "USD", "limit": 0}"#,
            ))
            .await
            .expect("The monthly series call should succeed");

        assert!(result.result.contains("Recent 2 entries"));
        assert!(!result.result.contains("more entries available"));
        api.stop().await;
    }

    #[tokio::test]
    async fn missing_time_series_says_so() {
        let api = MockHttpApi::serving(
            "GET",
            QUERY_PATH,
            MockResponse::json(json!({"Meta Data": {"2. Digital Currency Code": "BTC"}})),
        )
        .await;

        let result = tool_for(&api)
            .execute(&tool_call(
                r#"{"function": "DIGITAL_CURRENCY_DAILY", "from_currency": "BTC", "to_currency": "USD"}"#,
            ))
            .await
            .expect("A metadata-only body is not an error");

        assert!(result.result.contains("No time series data found."));
        api.stop().await;
    }

    #[tokio::test]
    async fn http_error_status_is_reported_with_body() {
        let api = MockHttpApi::serving(
            "GET",
            QUERY_PATH,
            MockResponse::error(503, "upstream unavailable"),
        )
        .await;

        let error = tool_for(&api)
            .execute(&tool_call(
                r#"{"from_currency": "BTC", "to_currency": "USD"}"#,
            ))
            .await
            .expect_err("A 503 must fail the call");

        let message = error.to_string();
        assert!(message.contains("503"), "{}", message);
        assert!(message.contains("upstream unavailable"), "{}", message);
        api.stop().await;
    }

    #[tokio::test]
    async fn api_level_error_message_is_surfaced() {
        let api = MockHttpApi::serving(
            "GET",
            QUERY_PATH,
            MockResponse::json(json!({"Error Message": "Invalid API call"})),
        )
        .await;

        let error = tool_for(&api)
            .execute(&tool_call(
                r#"{"from_currency": "BTC", "to_currency": "USD"}"#,
            ))
            .await
            .expect_err("A 200 carrying Error Message must still fail");

        assert_eq!(
            error.to_string(),
            "Alpha Vantage API error: Invalid API call"
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn rate_limit_note_fails_but_other_notes_do_not() {
        let api = MockHttpApi::start().await;
        api.on_sequence(
            "GET",
            QUERY_PATH,
            vec![
                MockResponse::json(json!({"Note": "Thank you for using Alpha Vantage! Our standard API rate limit is 25 requests per day."})),
                MockResponse::json(json!({
                    "Note": "This is a purely informational note",
                    "Realtime Currency Exchange Rate": {"5. Exchange Rate": "1.23"}
                })),
            ],
        );
        let tool = tool_for(&api);
        let call = tool_call(r#"{"from_currency": "BTC", "to_currency": "USD"}"#);

        let error = tool
            .execute(&call)
            .await
            .expect_err("A rate-limit Note must fail the call");
        assert!(
            error
                .to_string()
                .starts_with("Alpha Vantage API limit reached:"),
            "{}",
            error
        );

        // A Note without "Thank you" is only a warning, so the call goes through.
        let result = tool
            .execute(&call)
            .await
            .expect("A non-rate-limit Note must not fail the call");
        assert!(result.result.contains("Rate: 1.23"));

        assert_eq!(api.call_count(), 2);
        api.stop().await;
    }

    #[tokio::test]
    async fn malformed_json_is_reported_as_a_parse_failure() {
        let api = MockHttpApi::serving(
            "GET",
            QUERY_PATH,
            MockResponse::raw(200, "application/json", "{\"Realtime\": "),
        )
        .await;

        let error = tool_for(&api)
            .execute(&tool_call(
                r#"{"from_currency": "BTC", "to_currency": "USD"}"#,
            ))
            .await
            .expect_err("A truncated body must fail the call");

        assert_eq!(
            error.to_string(),
            "Failed to parse Alpha Vantage API response"
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn a_missing_api_key_fails_before_any_request() {
        let api = MockHttpApi::serving("GET", QUERY_PATH, MockResponse::json(json!({}))).await;
        let tool = CryptoTool::with_base_url(api.url(QUERY_PATH), None);

        let error = tool
            .execute(&tool_call(
                r#"{"from_currency": "BTC", "to_currency": "USD"}"#,
            ))
            .await
            .expect_err("Without a key the call must fail");

        assert_eq!(
            error.to_string(),
            "ALPHA_ADVANTAGE_KEY environment variable not set"
        );
        assert_eq!(api.call_count(), 0, "No request should have been attempted");
        api.stop().await;
    }

    #[tokio::test]
    async fn bad_arguments_fail_before_any_request() {
        let api = MockHttpApi::serving("GET", QUERY_PATH, MockResponse::json(json!({}))).await;
        let tool = tool_for(&api);

        let cases = [
            ("not json at all", "Failed to parse crypto tool arguments"),
            (
                r#"{"to_currency": "USD"}"#,
                "Missing required 'from_currency' parameter",
            ),
            (
                r#"{"from_currency": "BTC"}"#,
                "Missing required 'to_currency' parameter",
            ),
        ];
        for (arguments, expected) in cases {
            let error = tool
                .execute(&tool_call(arguments))
                .await
                .expect_err(arguments);
            assert_eq!(error.to_string(), expected, "for arguments {}", arguments);
        }
        assert_eq!(api.call_count(), 0);
        api.stop().await;
    }

    #[tokio::test]
    async fn an_unknown_function_fails_before_any_request() {
        let api = MockHttpApi::serving("GET", QUERY_PATH, MockResponse::json(json!({}))).await;

        let error = tool_for(&api)
            .execute(&tool_call(
                r#"{"function": "DIGITAL_CURRENCY_HOURLY", "from_currency": "BTC", "to_currency": "USD"}"#,
            ))
            .await
            .expect_err("An unsupported function must fail");

        assert!(
            error
                .to_string()
                .starts_with("Invalid function 'DIGITAL_CURRENCY_HOURLY'."),
            "{}",
            error
        );
        assert_eq!(api.call_count(), 0, "No request should have been attempted");
        api.stop().await;
    }
}
