use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest;
use serde_json::json;

/// Currency tool for fetching exchange rates from NBP (National Bank of Poland)
pub struct CurrencyTool {
    metadata: ToolMetadata,
    client: reqwest::Client,
}

impl CurrencyTool {
    /// Create a new instance of the currency tool
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                id: "5".to_string(), // Next ID after WeatherTool (Assuming Weather is 4)
                name: "currency_check".to_string(),
                category: ToolCategory::Financial,
                tool_type: ToolType::Currency,
            },
            client: reqwest::Client::new(),
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
        let base_url = "https://api.nbp.pl/api/exchangerates";
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
            tool_name: "currency_check".to_string(),
            result,
        })
    }
}
