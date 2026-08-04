use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest;
use serde_json::json;
use std::env;

/// The real Alpha Vantage query endpoint, used unless a test overrides it.
const ALPHA_VANTAGE_URL: &str = "https://www.alphavantage.co/query";

/// Stock tool for fetching stock market data from Alpha Vantage API
pub struct StockTool {
    metadata: ToolMetadata,
    client: reqwest::Client,
    api_key: Option<String>,
    /// Query endpoint to talk to. Always the real Alpha Vantage one in
    /// production; tests point it at a loopback mock instead.
    base_url: String,
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

        let base_url = &self.base_url;

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
            tool_call_id: None,
            tool_name: "stock_data".to_string(),
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

    const QUERY_PATH: &str = "/query";

    fn tool_call(arguments: &str) -> ToolCall {
        ToolCall {
            id: "call_stock".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "stock_data".to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    fn tool_for(api: &MockHttpApi) -> StockTool {
        StockTool::with_base_url(api.url(QUERY_PATH), Some("test-key"))
    }

    fn daily_body() -> serde_json::Value {
        json!({
            "Meta Data": {
                "2. Symbol": "NVDA",
                "3. Last Refreshed": "2026-08-03"
            },
            "Time Series (Daily)": {
                "2026-08-01": {"1. open": "1.0", "2. high": "1.5", "3. low": "0.5", "4. close": "1.2", "5. volume": "100"},
                "2026-08-02": {"1. open": "2.0", "2. high": "2.5", "3. low": "1.5", "4. close": "2.2", "5. volume": "200"},
                "2026-08-03": {"1. open": "3.0", "2. high": "3.5", "3. low": "2.5", "4. close": "3.2", "5. volume": "300"}
            }
        })
    }

    #[test]
    fn metadata_and_function_definition_describe_the_stock_tool() {
        let tool = StockTool::new();
        assert_eq!(tool.metadata().id, "6");
        assert_eq!(tool.metadata().category, ToolCategory::Financial);
        assert_eq!(tool.metadata().tool_type, ToolType::Stock);

        let def = tool.get_function_definition();
        assert_eq!(def["name"], "stock_data");
        assert_eq!(def["parameters"]["required"], json!(["symbol"]));
        assert_eq!(
            def["parameters"]["properties"]["function"]["default"],
            "TIME_SERIES_DAILY"
        );
    }

    #[test]
    fn availability_follows_the_api_key() {
        assert!(StockTool::with_base_url("http://unused", Some("k")).is_available());
        assert!(!StockTool::with_base_url("http://unused", None).is_available());
    }

    #[tokio::test]
    async fn daily_series_sends_the_expected_query_and_formats_ohlcv() {
        let api = MockHttpApi::serving("GET", QUERY_PATH, MockResponse::json(daily_body())).await;

        let result = tool_for(&api)
            .execute(&tool_call(
                r#"{"symbol": "nvda", "outputsize": "full", "limit": 2}"#,
            ))
            .await
            .expect("The daily series call should succeed");

        // The symbol is upper-cased and outputsize is only sent when supplied.
        let request = api.only_request();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, QUERY_PATH);
        assert_eq!(
            request.query_params(),
            vec![
                ("function".to_string(), "TIME_SERIES_DAILY".to_string()),
                ("symbol".to_string(), "NVDA".to_string()),
                ("apikey".to_string(), "test-key".to_string()),
                ("outputsize".to_string(), "full".to_string()),
            ]
        );
        assert_eq!(request.header("user-agent"), Some("ai_tools/1.0"));

        assert_eq!(result.tool_name, "stock_data");
        assert!(result.tool_call_id.is_none());
        assert!(result.result.contains("Stock Data for NVDA"));
        assert!(result.result.contains("Last Refreshed: 2026-08-03"));
        assert!(result.result.contains("Recent 2 entries (Chronological):"));
        assert!(result
            .result
            .contains("Open: $2.0 | High: $2.5 | Low: $1.5 | Close: $2.2"));
        assert!(result.result.contains("Volume: 300"));
        // Only the two most recent, oldest-first, with the rest counted.
        assert!(!result.result.contains("**2026-08-01**"));
        let second = result
            .result
            .find("**2026-08-02**")
            .expect("2026-08-02 shown");
        let third = result
            .result
            .find("**2026-08-03**")
            .expect("2026-08-03 shown");
        assert!(second < third, "Entries must be chronological");
        assert!(result.result.contains("_...and 1 more entries available_"));

        api.stop().await;
    }

    #[tokio::test]
    async fn outputsize_is_omitted_when_not_requested() {
        let api = MockHttpApi::serving("GET", QUERY_PATH, MockResponse::json(daily_body())).await;

        tool_for(&api)
            .execute(&tool_call(r#"{"symbol": "NVDA"}"#))
            .await
            .expect("The call should succeed");

        let request = api.only_request();
        assert!(
            request.query_param("outputsize").is_none(),
            "outputsize must not be sent when the caller did not ask for one: {}",
            request.query
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn weekly_and_monthly_read_their_own_series_keys() {
        let api = MockHttpApi::start().await;
        api.on_sequence(
            "GET",
            QUERY_PATH,
            vec![
                MockResponse::json(
                    json!({"Weekly Time Series": {"2026-08-03": {"4. close": "9.9"}}}),
                ),
                MockResponse::json(
                    json!({"Monthly Time Series": {"2026-07-31": {"4. close": "8.8"}}}),
                ),
            ],
        );
        let tool = tool_for(&api);

        let weekly = tool
            .execute(&tool_call(
                r#"{"symbol": "NVDA", "function": "TIME_SERIES_WEEKLY"}"#,
            ))
            .await
            .expect("The weekly call should succeed");
        assert!(weekly.result.contains("Close: $9.9"), "{}", weekly.result);

        let monthly = tool
            .execute(&tool_call(
                r#"{"symbol": "NVDA", "function": "TIME_SERIES_MONTHLY"}"#,
            ))
            .await
            .expect("The monthly call should succeed");
        assert!(monthly.result.contains("Close: $8.8"), "{}", monthly.result);

        let functions: Vec<Option<String>> = api
            .requests()
            .iter()
            .map(|request| request.query_param("function"))
            .collect();
        assert_eq!(
            functions,
            vec![
                Some("TIME_SERIES_WEEKLY".to_string()),
                Some("TIME_SERIES_MONTHLY".to_string())
            ]
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn limit_zero_shows_every_entry_and_missing_values_fall_back() {
        let api = MockHttpApi::serving(
            "GET",
            QUERY_PATH,
            MockResponse::json(
                json!({"Time Series (Daily)": {"2026-08-01": {}, "2026-08-02": {}}}),
            ),
        )
        .await;

        let result = tool_for(&api)
            .execute(&tool_call(r#"{"symbol": "NVDA", "limit": 0}"#))
            .await
            .expect("The call should succeed");

        assert!(result.result.contains("Recent 2 entries"));
        assert!(!result.result.contains("more entries available"));
        assert!(result
            .result
            .contains("Open: $N/A | High: $N/A | Low: $N/A | Close: $N/A"));
        assert!(result.result.contains("Volume: N/A"));
        api.stop().await;
    }

    #[tokio::test]
    async fn a_body_without_a_time_series_says_so() {
        let api = MockHttpApi::serving("GET", QUERY_PATH, MockResponse::json(json!({}))).await;

        let result = tool_for(&api)
            .execute(&tool_call(r#"{"symbol": "NVDA"}"#))
            .await
            .expect("An empty body is not an error");

        assert_eq!(result.result, "No time series data found in response.\n");
        api.stop().await;
    }

    #[tokio::test]
    async fn http_error_status_is_reported_with_body() {
        let api =
            MockHttpApi::serving("GET", QUERY_PATH, MockResponse::error(429, "slow down")).await;

        let error = tool_for(&api)
            .execute(&tool_call(r#"{"symbol": "NVDA"}"#))
            .await
            .expect_err("A 429 must fail the call");

        let message = error.to_string();
        assert!(message.contains("429"), "{}", message);
        assert!(message.contains("slow down"), "{}", message);
        api.stop().await;
    }

    #[tokio::test]
    async fn api_level_error_and_any_note_are_both_failures() {
        // Unlike the crypto tool, this one treats *every* Note as a rate limit.
        let api = MockHttpApi::start().await;
        api.on_sequence(
            "GET",
            QUERY_PATH,
            vec![
                MockResponse::json(json!({"Error Message": "Invalid API call"})),
                MockResponse::json(json!({"Note": "just a friendly note"})),
            ],
        );
        let tool = tool_for(&api);
        let call = tool_call(r#"{"symbol": "NVDA"}"#);

        assert_eq!(
            tool.execute(&call)
                .await
                .expect_err("Error Message must fail")
                .to_string(),
            "Alpha Vantage API error: Invalid API call"
        );
        assert_eq!(
            tool.execute(&call)
                .await
                .expect_err("A Note must fail")
                .to_string(),
            "Alpha Vantage API limit reached: just a friendly note"
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn malformed_json_is_reported_as_a_parse_failure() {
        let api = MockHttpApi::serving(
            "GET",
            QUERY_PATH,
            MockResponse::raw(200, "application/json", "<html>not json</html>"),
        )
        .await;

        let error = tool_for(&api)
            .execute(&tool_call(r#"{"symbol": "NVDA"}"#))
            .await
            .expect_err("A non-JSON body must fail the call");

        assert_eq!(
            error.to_string(),
            "Failed to parse Alpha Vantage API response"
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn bad_arguments_and_a_missing_key_fail_before_any_request() {
        let api = MockHttpApi::serving("GET", QUERY_PATH, MockResponse::json(daily_body())).await;
        let tool = tool_for(&api);

        assert_eq!(
            tool.execute(&tool_call("}{"))
                .await
                .expect_err("Unparseable arguments must fail")
                .to_string(),
            "Failed to parse stock tool arguments"
        );
        assert_eq!(
            tool.execute(&tool_call(r#"{"function": "TIME_SERIES_DAILY"}"#))
                .await
                .expect_err("A missing symbol must fail")
                .to_string(),
            "Missing required 'symbol' parameter"
        );
        assert!(tool
            .execute(&tool_call(
                r#"{"symbol": "NVDA", "function": "TIME_SERIES_HOURLY"}"#
            ))
            .await
            .expect_err("An unsupported function must fail")
            .to_string()
            .starts_with("Invalid function 'TIME_SERIES_HOURLY'."));

        let keyless = StockTool::with_base_url(api.url(QUERY_PATH), None);
        assert_eq!(
            keyless
                .execute(&tool_call(r#"{"symbol": "NVDA"}"#))
                .await
                .expect_err("Without a key the call must fail")
                .to_string(),
            "ALPHA_ADVANTAGE_KEY environment variable not set"
        );

        assert_eq!(api.call_count(), 0, "Nothing should have reached the API");
        api.stop().await;
    }
}
