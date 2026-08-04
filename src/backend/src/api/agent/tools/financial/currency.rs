use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest;
use serde_json::json;

/// The real NBP exchange-rates API root, used unless a test overrides it.
const NBP_BASE_URL: &str = "https://api.nbp.pl/api/exchangerates";

/// Currency tool for fetching exchange rates from NBP (National Bank of Poland)
pub struct CurrencyTool {
    metadata: ToolMetadata,
    client: reqwest::Client,
    /// API root to talk to. Always the real NBP one in production; tests point it
    /// at a loopback mock instead.
    base_url: String,
}

impl CurrencyTool {
    /// Create a new instance of the currency tool
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                id: "5".to_string(), // Next ID after WeatherTool (Assuming Weather is 4)
                name: "Currency Exchange".to_string(),
                description: "Check currency exchange rates via NBP".to_string(),
                category: ToolCategory::Financial,
                tool_type: ToolType::Currency,
            },
            client: reqwest::Client::new(),
            base_url: NBP_BASE_URL.to_string(),
        }
    }

    /// A tool pointed at `base_url` instead of the real NBP API, so the
    /// request/response handling can be driven without the network.
    #[cfg(test)]
    pub(crate) fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Self::new()
        }
    }

    /// Fetch currency data from NBP API
    async fn fetch_currency_data(
        &self,
        table: &str,
        code: Option<&str>,
        date: Option<&str>,
        last: Option<u64>,
    ) -> Result<serde_json::Value> {
        let base_url = &self.base_url;
        let format = "?format=json";

        let url = if let Some(currency_code) = code {
            // Single currency query
            if let Some(d) = date {
                format!(
                    "{}/rates/{}/{}/{}/{}",
                    base_url, table, currency_code, d, format
                )
            } else if let Some(n) = last {
                format!(
                    "{}/rates/{}/{}/last/{}/{}",
                    base_url, table, currency_code, n, format
                )
            } else {
                format!("{}/rates/{}/{}/{}", base_url, table, currency_code, format)
            }
        } else {
            // Whole table query
            if let Some(d) = date {
                format!("{}/tables/{}/{}/{}", base_url, table, d, format)
            } else if let Some(n) = last {
                format!("{}/tables/{}/last/{}/{}", base_url, table, n, format)
            } else {
                format!("{}/tables/{}/{}", base_url, table, format)
            }
        };

        println!("\x1b[33m💰 Fetching currency data from: {}\x1b[0m", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to request currency data from NBP API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "NBP API returned error {}: {}",
                status,
                error_text
            ));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse NBP API response")?;

        Ok(data)
    }

    fn format_currency_response(&self, data: &serde_json::Value, table: &str) -> Result<String> {
        let mut result = String::new();

        // Check if it's a rates array (single currency history) or table array
        if let Some(rates_wrapper) = data.as_object() {
            // Single currency response usually looks like: { "table": "A", "currency": "dolar amerykański", "code": "USD", "rates": [...] }
            if let Some(currency) = rates_wrapper.get("currency").and_then(|c| c.as_str()) {
                let code = rates_wrapper
                    .get("code")
                    .and_then(|c| c.as_str())
                    .unwrap_or("???");
                result.push_str(&format!(
                    "💱 **Exchange Rates for {} ({})**\n",
                    currency, code
                ));

                if let Some(rates) = rates_wrapper.get("rates").and_then(|r| r.as_array()) {
                    for rate in rates {
                        let date = rate
                            .get("effectiveDate")
                            .and_then(|d| d.as_str())
                            .unwrap_or("Unknown Date");
                        if let Some(mid) = rate.get("mid").and_then(|m| m.as_f64()) {
                            result.push_str(&format!("  📅 {}: **{:.4} PLN**\n", date, mid));
                        } else {
                            // Table C has bid/ask
                            let bid = rate.get("bid").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let ask = rate.get("ask").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            result.push_str(&format!(
                                "  📅 {}: Bid: **{:.4} PLN**, Ask: **{:.4} PLN**\n",
                                date, bid, ask
                            ));
                        }
                    }
                }
            } else if let Some(array) = data.as_array() {
                // It might be an array of tables (e.g. last N tables)
                for item in array {
                    let effective_date = item
                        .get("effectiveDate")
                        .and_then(|d| d.as_str())
                        .unwrap_or("Unknown Date");
                    result.push_str(&format!(
                        "📅 **Table {} from {}**\n",
                        table.to_uppercase(),
                        effective_date
                    ));

                    if let Some(rates) = item.get("rates").and_then(|r| r.as_array()) {
                        for rate in rates {
                            let code = rate.get("code").and_then(|c| c.as_str()).unwrap_or("???");
                            let currency = rate
                                .get("currency")
                                .and_then(|c| c.as_str())
                                .unwrap_or("Unknown");

                            if let Some(mid) = rate.get("mid").and_then(|m| m.as_f64()) {
                                result.push_str(&format!(
                                    "  • {} ({}): **{:.4} PLN**\n",
                                    code, currency, mid
                                ));
                            } else {
                                let bid = rate.get("bid").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                let ask = rate.get("ask").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                result.push_str(&format!(
                                    "  • {} ({}): Bid: **{:.4}**, Ask: **{:.4}**\n",
                                    code, currency, bid, ask
                                ));
                            }
                        }
                    }
                    result.push('\n');
                }
            }
        } else if let Some(array) = data.as_array() {
            // Top-level array (e.g. list of tables)
            for item in array {
                let effective_date = item
                    .get("effectiveDate")
                    .and_then(|d| d.as_str())
                    .unwrap_or("Unknown Date");
                result.push_str(&format!(
                    "📅 **Table {} from {}**\n",
                    table.to_uppercase(),
                    effective_date
                ));

                if let Some(rates) = item.get("rates").and_then(|r| r.as_array()) {
                    for rate in rates {
                        let code = rate.get("code").and_then(|c| c.as_str()).unwrap_or("???");
                        let currency = rate
                            .get("currency")
                            .and_then(|c| c.as_str())
                            .unwrap_or("Unknown");

                        if let Some(mid) = rate.get("mid").and_then(|m| m.as_f64()) {
                            result.push_str(&format!(
                                "  • {} ({}): **{:.4} PLN**\n",
                                code, currency, mid
                            ));
                        } else {
                            let bid = rate.get("bid").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let ask = rate.get("ask").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            result.push_str(&format!(
                                "  • {} ({}): Bid: **{:.4}**, Ask: **{:.4}**\n",
                                code, currency, bid, ask
                            ));
                        }
                    }
                }
                result.push('\n');
            }
        }

        if result.is_empty() {
            Ok("No currency data found or format not recognized.".to_string())
        } else {
            Ok(result)
        }
    }
}

#[async_trait]
impl AgentTool for CurrencyTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "currency_check",
            "description": "Check currency exchange rates via NBP (National Bank of Poland). Supports current rates, historical rates, and whole tables. Table 'A' is for mid rates of foreign currencies, 'B' for mid rates of unconvertible currencies, 'C' for bid/ask rates.",
            "parameters": {
                "type": "object",
                "properties": {
                    "code": {
                        "type": "string",
                        "description": "3-letter currency code (e.g. 'USD', 'EUR'). If omitted, fetches the whole table."
                    },
                    "table": {
                        "type": "string",
                        "description": "Table type: 'A' (mid rates), 'B' (other mid rates), 'C' (bid/ask). Defaults to 'A'.",
                        "enum": ["A", "B", "C"],
                        "default": "A"
                    },
                    "date": {
                         "type": "string",
                         "description": "Specific date in YYYY-MM-DD format."
                    },
                    "last": {
                        "type": "integer",
                         "description": "Number of last records to fetch (e.g. last 10 rates)."
                    }
                },
                "required": []
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse currency tool arguments")?;

        let code = args.get("code").and_then(|v| v.as_str());
        let table = args.get("table").and_then(|v| v.as_str()).unwrap_or("A");
        let date = args.get("date").and_then(|v| v.as_str());
        let last = args.get("last").and_then(|v| v.as_u64());

        // Basic validation
        if let Some(d) = date {
            // simple regex or length check could work, but let's just trust NBP to return 400 if bad
            // Actually, good to do a basic length check
            if d.len() != 10 {
                return Err(anyhow::anyhow!("Date must be in YYYY-MM-DD format"));
            }
        }

        let data = self.fetch_currency_data(table, code, date, last).await?;
        let result = self.format_currency_response(&data, table)?;

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "currency_check".to_string(),
            result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::core::types::FunctionCall;
    use crate::test_support::{MockHttpApi, MockResponse};

    /// The NBP API root, mirrored under the mock so recorded paths match the
    /// real service's shape.
    const ROOT: &str = "/api/exchangerates";

    fn tool_call(arguments: &str) -> ToolCall {
        ToolCall {
            id: "call_currency".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "currency_check".to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    fn tool_for(api: &MockHttpApi) -> CurrencyTool {
        CurrencyTool::with_base_url(api.url(ROOT))
    }

    #[test]
    fn metadata_and_function_definition_describe_the_currency_tool() {
        let tool = CurrencyTool::new();
        assert_eq!(tool.metadata().id, "5");
        assert_eq!(tool.metadata().category, ToolCategory::Financial);
        assert_eq!(tool.metadata().tool_type, ToolType::Currency);
        assert!(tool.is_available());

        let def = tool.get_function_definition();
        assert_eq!(def["name"], "currency_check");
        assert_eq!(def["parameters"]["required"], json!([]));
        assert_eq!(def["parameters"]["properties"]["table"]["default"], "A");
    }

    #[tokio::test]
    async fn a_single_currency_uses_the_rates_path_and_formats_mid_rates() {
        // Note the trailing slash before the query string: the tool joins its
        // "?format=json" suffix as if it were a path segment, so this really is
        // the URL the live API receives.
        let api = MockHttpApi::serving(
            "GET",
            &format!("{}/rates/A/USD/", ROOT),
            MockResponse::json(json!({
                "table": "A",
                "currency": "dolar amerykański",
                "code": "USD",
                "rates": [
                    {"no": "148/A/NBP/2026", "effectiveDate": "2026-08-01", "mid": 3.9812},
                    {"no": "149/A/NBP/2026", "effectiveDate": "2026-08-02", "mid": 4.0}
                ]
            })),
        )
        .await;

        let result = tool_for(&api)
            .execute(&tool_call(r#"{"code": "USD"}"#))
            .await
            .expect("The single-currency call should succeed");

        let request = api.only_request();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, format!("{}/rates/A/USD/", ROOT));
        assert_eq!(request.query, "format=json");

        assert_eq!(result.tool_name, "currency_check");
        assert!(result.tool_call_id.is_none());
        assert!(result
            .result
            .contains("Exchange Rates for dolar amerykański (USD)"));
        assert!(result.result.contains("2026-08-01: **3.9812 PLN**"));
        // Four decimal places even when the source value has fewer.
        assert!(result.result.contains("2026-08-02: **4.0000 PLN**"));
        api.stop().await;
    }

    #[tokio::test]
    async fn table_c_rates_are_rendered_as_bid_and_ask() {
        let api = MockHttpApi::serving(
            "GET",
            &format!("{}/rates/C/EUR/last/2/", ROOT),
            MockResponse::json(json!({
                "currency": "euro",
                "code": "EUR",
                "rates": [{"effectiveDate": "2026-08-02", "bid": 4.2, "ask": 4.3}]
            })),
        )
        .await;

        let result = tool_for(&api)
            .execute(&tool_call(r#"{"code": "EUR", "table": "C", "last": 2}"#))
            .await
            .expect("The table C call should succeed");

        assert_eq!(
            api.only_request().path,
            format!("{}/rates/C/EUR/last/2/", ROOT)
        );
        assert!(result
            .result
            .contains("2026-08-02: Bid: **4.2000 PLN**, Ask: **4.3000 PLN**"));
        api.stop().await;
    }

    #[tokio::test]
    async fn a_dated_single_currency_query_puts_the_date_in_the_path() {
        let api = MockHttpApi::serving(
            "GET",
            &format!("{}/rates/A/CHF/2026-07-15/", ROOT),
            MockResponse::json(json!({
                "currency": "frank szwajcarski",
                "code": "CHF",
                "rates": [{"effectiveDate": "2026-07-15", "mid": 4.5}]
            })),
        )
        .await;

        let result = tool_for(&api)
            .execute(&tool_call(r#"{"code": "CHF", "date": "2026-07-15"}"#))
            .await
            .expect("The dated call should succeed");

        assert_eq!(
            api.only_request().path,
            format!("{}/rates/A/CHF/2026-07-15/", ROOT)
        );
        assert!(result.result.contains("2026-07-15: **4.5000 PLN**"));
        api.stop().await;
    }

    #[tokio::test]
    async fn a_whole_table_query_lists_every_rate() {
        let api = MockHttpApi::serving(
            "GET",
            &format!("{}/tables/A/", ROOT),
            MockResponse::json(json!([{
                "table": "A",
                "effectiveDate": "2026-08-03",
                "rates": [
                    {"currency": "dolar amerykański", "code": "USD", "mid": 3.9},
                    {"currency": "korona czeska", "code": "CZK", "bid": 0.17, "ask": 0.18}
                ]
            }])),
        )
        .await;

        let result = tool_for(&api)
            .execute(&tool_call("{}"))
            .await
            .expect("The whole-table call should succeed");

        assert_eq!(api.only_request().path, format!("{}/tables/A/", ROOT));
        assert!(result.result.contains("Table A from 2026-08-03"));
        assert!(result
            .result
            .contains("USD (dolar amerykański): **3.9000 PLN**"));
        // A rate without "mid" falls back to the bid/ask rendering.
        assert!(result
            .result
            .contains("CZK (korona czeska): Bid: **0.1700**, Ask: **0.1800**"));
        api.stop().await;
    }

    #[tokio::test]
    async fn a_last_n_table_query_uses_the_last_path_segment() {
        let api = MockHttpApi::serving(
            "GET",
            &format!("{}/tables/B/last/3/", ROOT),
            MockResponse::json(json!([{"effectiveDate": "2026-08-03", "rates": []}])),
        )
        .await;

        let result = tool_for(&api)
            .execute(&tool_call(r#"{"table": "B", "last": 3}"#))
            .await
            .expect("The last-N table call should succeed");

        assert_eq!(
            api.only_request().path,
            format!("{}/tables/B/last/3/", ROOT)
        );
        assert!(result.result.contains("Table B from 2026-08-03"));
        api.stop().await;
    }

    #[tokio::test]
    async fn a_dated_table_query_puts_the_date_in_the_path() {
        let api = MockHttpApi::serving(
            "GET",
            &format!("{}/tables/A/2026-07-15/", ROOT),
            MockResponse::json(json!([{"effectiveDate": "2026-07-15", "rates": []}])),
        )
        .await;

        tool_for(&api)
            .execute(&tool_call(r#"{"date": "2026-07-15"}"#))
            .await
            .expect("The dated table call should succeed");

        assert_eq!(
            api.only_request().path,
            format!("{}/tables/A/2026-07-15/", ROOT)
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn an_unrecognised_body_reports_no_data_rather_than_failing() {
        let api = MockHttpApi::serving(
            "GET",
            &format!("{}/tables/A/", ROOT),
            MockResponse::json(json!({"unexpected": "shape"})),
        )
        .await;

        let result = tool_for(&api)
            .execute(&tool_call("{}"))
            .await
            .expect("An unknown shape is not an error");

        assert_eq!(
            result.result,
            "No currency data found or format not recognized."
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn a_404_from_nbp_is_reported_with_its_body() {
        let api = MockHttpApi::serving(
            "GET",
            &format!("{}/rates/A/XXX/", ROOT),
            MockResponse::error(404, "404 NotFound - Not Found - Brak danych"),
        )
        .await;

        let error = tool_for(&api)
            .execute(&tool_call(r#"{"code": "XXX"}"#))
            .await
            .expect_err("A 404 must fail the call");

        let message = error.to_string();
        assert!(
            message.starts_with("NBP API returned error 404"),
            "{}",
            message
        );
        assert!(message.contains("Brak danych"), "{}", message);
        api.stop().await;
    }

    #[tokio::test]
    async fn malformed_json_is_reported_as_a_parse_failure() {
        let api = MockHttpApi::serving(
            "GET",
            &format!("{}/tables/A/", ROOT),
            MockResponse::raw(200, "application/json", "[{"),
        )
        .await;

        let error = tool_for(&api)
            .execute(&tool_call("{}"))
            .await
            .expect_err("A truncated body must fail the call");

        assert_eq!(error.to_string(), "Failed to parse NBP API response");
        api.stop().await;
    }

    #[tokio::test]
    async fn an_unreachable_api_is_reported_as_a_request_failure() {
        // Port 1 is privileged and never bound, so the connection is refused.
        let tool = CurrencyTool::with_base_url("http://127.0.0.1:1/api/exchangerates");

        let error = tool
            .execute(&tool_call("{}"))
            .await
            .expect_err("An unreachable API must fail the call");

        assert_eq!(
            error.to_string(),
            "Failed to request currency data from NBP API"
        );
    }

    #[tokio::test]
    async fn bad_arguments_fail_before_any_request() {
        let api = MockHttpApi::serving(
            "GET",
            &format!("{}/tables/A/", ROOT),
            MockResponse::json(json!([])),
        )
        .await;
        let tool = tool_for(&api);

        assert_eq!(
            tool.execute(&tool_call("not json"))
                .await
                .expect_err("Unparseable arguments must fail")
                .to_string(),
            "Failed to parse currency tool arguments"
        );
        assert_eq!(
            tool.execute(&tool_call(r#"{"date": "15-07-2026 12:00"}"#))
                .await
                .expect_err("A malformed date must fail")
                .to_string(),
            "Date must be in YYYY-MM-DD format"
        );

        assert_eq!(api.call_count(), 0, "Nothing should have reached the API");
        api.stop().await;
    }
}
