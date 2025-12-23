use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use reqwest;
use serde_json::json;
use std::env;
use url::Url;

/// Weather tool for fetching current weather data
pub struct WeatherTool {
    metadata: ToolMetadata,
    client: reqwest::Client,
    api_key: String,
}

impl WeatherTool {
    /// Create a new instance of the weather tool
    pub fn new() -> Self {
        let api_key = env::var("OPENWEATHER_API_KEY").unwrap_or_default();

        Self {
            metadata: ToolMetadata {
                id: "weather_current".to_string(),
                name: "weather_current".to_string(),
                category: ToolCategory::Utility,
                tool_type: ToolType::Weather,
            },
            client: reqwest::Client::new(),
            api_key,
        }
    }

    /// Fetch weather data using city name or coordinates
    async fn fetch_weather_data(
        &self,
        city: Option<&str>,
        state: Option<&str>,
        country: Option<&str>,
        lat: Option<f64>,
        lon: Option<f64>,
        units: &str,
    ) -> Result<serde_json::Value> {
        let mut url = Url::parse("https://api.openweathermap.org/data/2.5/weather")
            .context("Failed to parse base URL")?;

        if let (Some(lat_val), Some(lon_val)) = (lat, lon) {
            // Use coordinates directly
            url.query_pairs_mut()
                .append_pair("lat", &lat_val.to_string())
                .append_pair("lon", &lon_val.to_string())
                .append_pair("units", units)
                .append_pair("appid", &self.api_key);
        } else if let Some(city_name) = city {
            // Build city query string
            let mut query = city_name.to_string();
            if let Some(state_val) = state {
                if !state_val.is_empty() {
                    query.push_str(&format!(",{}", state_val));
                }
            }
            if let Some(country_val) = country {
                if !country_val.is_empty() {
                    query.push_str(&format!(",{}", country_val));
                }
            }
            url.query_pairs_mut()
                .append_pair("q", &query)
                .append_pair("units", units)
                .append_pair("appid", &self.api_key);
        } else {
            return Err(anyhow::anyhow!(
                "Either city name or both latitude and longitude must be provided"
            ));
        }

        let weather_url = url.to_string();

        println!("\x1b[33m🌤️ Fetching weather data...\x1b[0m");

        let weather_response = self
            .client
            .get(&weather_url)
            .send()
            .await
            .context("Failed to request weather data from OpenWeatherMap")?;

        // Check HTTP status
        if !weather_response.status().is_success() {
            let status = weather_response.status();
            let error_text = weather_response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Weather API returned error {}: {}",
                status,
                error_text
            ));
        }

        let weather_data: serde_json::Value = weather_response
            .json()
            .await
            .context("Failed to parse weather response")?;

        Ok(weather_data)
    }

    /// Format weather data into a readable response string
    fn format_weather_response(
        &self,
        weather_data: &serde_json::Value,
        units: &str,
    ) -> Result<String> {
        // Extract location info
        let city_name = weather_data["name"].as_str().unwrap_or("Unknown Location");
        let lat = weather_data["coord"]["lat"].as_f64().unwrap_or(0.0);
        let lon = weather_data["coord"]["lon"].as_f64().unwrap_or(0.0);

        // Extract main weather data
        let main = weather_data
            .get("main")
            .ok_or_else(|| anyhow::anyhow!("Missing 'main' field in weather response"))?;

        let temp = main["temp"]
            .as_f64()
            .ok_or_else(|| anyhow::anyhow!("Missing temperature in weather response"))?;
        let feels_like = main["feels_like"].as_f64().unwrap_or(temp);
        let humidity = main["humidity"].as_u64().unwrap_or(0);
        let pressure = main["pressure"].as_u64().unwrap_or(0);

        // Extract wind data
        let wind = weather_data.get("wind");
        let wind_speed = wind.and_then(|w| w["speed"].as_f64()).unwrap_or(0.0);
        let wind_deg = wind.and_then(|w| w["deg"].as_u64());

        // Get weather description
        let weather_array = weather_data
            .get("weather")
            .and_then(|w| w.as_array())
            .ok_or_else(|| anyhow::anyhow!("Missing weather conditions in response"))?;

        if weather_array.is_empty() {
            return Err(anyhow::anyhow!("Empty weather conditions array"));
        }

        let description = weather_array[0]["description"]
            .as_str()
            .unwrap_or("unknown");
        let icon = weather_array[0]["icon"].as_str().unwrap_or("");

        // Get temperature unit
        let temp_unit = match units {
            "metric" => "C",
            "imperial" => "F",
            _ => "K",
        };

        // Get wind speed unit
        let wind_unit = match units {
            "metric" => "m/s",
            "imperial" => "mph",
            _ => "m/s",
        };

        // Format wind direction if available
        let wind_info = if let Some(deg) = wind_deg {
            let direction = self.degrees_to_direction(deg);
            format!("{:.1} {} ({})", wind_speed, wind_unit, direction)
        } else {
            format!("{:.1} {}", wind_speed, wind_unit)
        };

        // Build response
        let mut result = format!(
            "🌡️ **Current Weather in {}** (Lat: {:.4}, Lon: {:.4})\n\n",
            city_name, lat, lon
        );

        result.push_str(&format!(
            "**Temperature:** {:.1}°{} (Feels like {:.1}°{})\n",
            temp, temp_unit, feels_like, temp_unit
        ));

        result.push_str(&format!(
            "**Conditions:** {}\n",
            description.to_string().to_titlecase()
        ));

        result.push_str(&format!(
            "**Humidity:** {}% | **Pressure:** {} hPa | **Wind:** {}\n",
            humidity, pressure, wind_info
        ));

        // Add visibility if available
        if let Some(visibility) = weather_data["visibility"].as_u64() {
            let visibility_km = visibility as f64 / 1000.0;
            result.push_str(&format!("**Visibility:** {:.1} km\n", visibility_km));
        }

        // Add cloudiness if available
        if let Some(clouds) = weather_data["clouds"]["all"].as_u64() {
            result.push_str(&format!("**Cloudiness:** {}%\n", clouds));
        }

        if !icon.is_empty() {
            result.push_str(&format!(
                "\n![Weather Icon](https://openweathermap.org/img/wn/{}@2x.png)\n",
                icon
            ));
        }

        Ok(result)
    }

    /// Convert wind direction degrees to cardinal direction
    fn degrees_to_direction(&self, degrees: u64) -> &'static str {
        let deg = degrees % 360;
        match deg {
            0..=22 | 338..=360 => "N",
            23..=67 => "NE",
            68..=112 => "E",
            113..=157 => "SE",
            158..=202 => "S",
            203..=247 => "SW",
            248..=292 => "W",
            293..=337 => "NW",
            _ => "N",
        }
    }
}

#[async_trait]
impl AgentTool for WeatherTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "weather_current",
            "description": "Get current weather data for a specific location. Use this when user asks about current weather conditions, temperature, humidity, or weather details for a specific city. If you know the latitude and longitude coordinates, provide them directly to skip geocoding. DO NOT use for forecasts or historical weather data.",
            "parameters": {
                "type": "object",
                "properties": {
                    "city": {
                        "type": "string",
                        "description": "Name of the city to check weather for. Required if latitude/longitude are not provided."
                    },
                    "latitude": {
                        "type": "number",
                        "description": "Latitude coordinate (-90 to 90). If provided with longitude, skips geocoding and fetches weather directly."
                    },
                    "longitude": {
                        "type": "number",
                        "description": "Longitude coordinate (-180 to 180). If provided with latitude, skips geocoding and fetches weather directly."
                    },
                    "state": {
                        "type": "string",
                        "description": "Optional state code (2-letter) for disambiguation when using city name"
                    },
                    "country": {
                        "type": "string",
                        "description": "Optional country code (2-letter) for disambiguation when using city name"
                    },
                    "units": {
                        "type": "string",
                        "description": "Units for temperature (standard, metric, or imperial). Defaults to metric.",
                        "default": "metric",
                        "enum": ["standard", "metric", "imperial"]
                    }
                },
                "required": []
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        // Parse arguments
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse weather tool arguments")?;

        let units = args
            .get("units")
            .and_then(|v| v.as_str())
            .unwrap_or("metric");

        // Validate units
        if !["standard", "metric", "imperial"].contains(&units) {
            return Err(anyhow::anyhow!(
                "Invalid units. Must be 'standard', 'metric', or 'imperial'"
            ));
        }

        // Extract parameters
        let city = args.get("city").and_then(|v| v.as_str());
        let state = args.get("state").and_then(|v| v.as_str());
        let country = args.get("country").and_then(|v| v.as_str());
        let lat = args.get("latitude").and_then(|v| v.as_f64());
        let lon = args.get("longitude").and_then(|v| v.as_f64());

        // Validate coordinates if provided
        if let Some(lat_val) = lat {
            if !(-90.0..=90.0).contains(&lat_val) {
                return Err(anyhow::anyhow!("Latitude must be between -90 and 90"));
            }
        }
        if let Some(lon_val) = lon {
            if !(-180.0..=180.0).contains(&lon_val) {
                return Err(anyhow::anyhow!("Longitude must be between -180 and 180"));
            }
        }

        // Log what we're doing
        if let (Some(lat_val), Some(lon_val)) = (lat, lon) {
            println!(
                "\x1b[36m🌐 Fetching weather for coordinates: lat={}, lon={}\x1b[0m",
                lat_val, lon_val
            );
        } else if let Some(city_name) = city {
            let location_display = if let Some(state_val) = state {
                if let Some(country_val) = country {
                    format!("{}, {}, {}", city_name, state_val, country_val)
                } else {
                    format!("{}, {}", city_name, state_val)
                }
            } else {
                city_name.to_string()
            };
            println!(
                "\x1b[36m🌐 Fetching weather for: {}\x1b[0m",
                location_display
            );
        } else {
            return Err(anyhow::anyhow!(
                "Either 'city' or both 'latitude' and 'longitude' must be provided"
            ));
        }

        // Fetch weather data
        let weather_data = self
            .fetch_weather_data(city, state, country, lat, lon, units)
            .await?;

        // Format and return result
        let result = self.format_weather_response(&weather_data, units)?;

        println!("\x1b[32m✅ Weather data retrieved successfully\x1b[0m");

        Ok(ToolCallResult {
            tool_name: "weather_check".to_string(),
            result,
        })
    }

    fn is_available(&self) -> bool {
        // Check if API key is set
        !self.api_key.is_empty()
    }
}

// Helper trait for string formatting
trait StringExt {
    fn to_titlecase(&self) -> String;
}

impl StringExt for str {
    fn to_titlecase(&self) -> String {
        let mut words = self.split_whitespace();
        let mut result = String::new();

        if let Some(first_word) = words.next() {
            result.push_str(&first_word[..1].to_uppercase());
            if first_word.len() > 1 {
                result.push_str(&first_word[1..].to_lowercase());
            }
        }

        for word in words {
            result.push(' ');
            result.push_str(&word[..1].to_uppercase());
            if word.len() > 1 {
                result.push_str(&word[1..].to_lowercase());
            }
        }

        result
    }
}

/// Weather tool for fetching 5-day forecast data
pub struct ForecastTool {
    metadata: ToolMetadata,
    client: reqwest::Client,
    api_key: String,
}

impl ForecastTool {
    /// Create a new instance of the forecast tool
    pub fn new() -> Self {
        let api_key = env::var("OPENWEATHER_API_KEY").unwrap_or_default();

        Self {
            metadata: ToolMetadata {
                id: "weather_forecast".to_string(),
                name: "weather_forecast".to_string(),
                category: ToolCategory::Utility,
                tool_type: ToolType::Weather,
            },
            client: reqwest::Client::new(),
            api_key,
        }
    }

    /// Fetch forecast data
    async fn fetch_forecast_data(
        &self,
        city: Option<&str>,
        state: Option<&str>,
        country: Option<&str>,
        lat: Option<f64>,
        lon: Option<f64>,
        units: &str,
    ) -> Result<serde_json::Value> {
        let mut url = Url::parse("https://api.openweathermap.org/data/2.5/forecast")
            .context("Failed to parse base URL")?;

        if let (Some(lat_val), Some(lon_val)) = (lat, lon) {
            url.query_pairs_mut()
                .append_pair("lat", &lat_val.to_string())
                .append_pair("lon", &lon_val.to_string())
                .append_pair("units", units)
                .append_pair("appid", &self.api_key);
        } else if let Some(city_name) = city {
            let mut query = city_name.to_string();
            if let Some(state_val) = state {
                if !state_val.is_empty() {
                    query.push_str(&format!(",{}", state_val));
                }
            }
            if let Some(country_val) = country {
                if !country_val.is_empty() {
                    query.push_str(&format!(",{}", country_val));
                }
            }
            url.query_pairs_mut()
                .append_pair("q", &query)
                .append_pair("units", units)
                .append_pair("appid", &self.api_key);
        } else {
            return Err(anyhow::anyhow!(
                "Either city name or both latitude and longitude must be provided"
            ));
        }

        let forecast_url = url.to_string();
        println!("\x1b[33m🗓️ Fetching 5-day forecast data...\x1b[0m");

        let response = self
            .client
            .get(&forecast_url)
            .send()
            .await
            .context("Failed to request forecast data")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Forecast API returned error {}: {}",
                status,
                error_text
            ));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse forecast response")?;

        Ok(data)
    }

    /// Format forecast data into a response
    fn format_forecast_response(
        &self,
        data: &serde_json::Value,
        target_date: Option<&str>,
        units: &str,
    ) -> Result<String> {
        let city_name = data["city"]["name"].as_str().unwrap_or("Unknown Location");
        let list = data["list"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing 'list' in forecast data"))?;

        let temp_unit = match units {
            "metric" => "C",
            "imperial" => "F",
            _ => "K",
        };

        let mut result = format!("🗓️ **5-Day Weather Forecast for {}**\n\n", city_name);

        if let Some(date_str) = target_date {
            // Filter for specific date (YYYY-MM-DD)
            result.push_str(&format!("**Forecast for {}:**\n", date_str));
            let mut found = false;

            for item in list {
                if let Some(dt_txt) = item["dt_txt"].as_str() {
                    if dt_txt.starts_with(date_str) {
                        found = true;
                        let time = &dt_txt[11..16]; // Extract HH:MM
                        let temp = item["main"]["temp"].as_f64().unwrap_or(0.0);
                        let weather = item["weather"][0]["description"]
                            .as_str()
                            .unwrap_or("unknown")
                            .to_titlecase();
                        let rain_prob = item["pop"].as_f64().map(|p| p * 100.0).unwrap_or(0.0);
                        let icon = item["weather"][0]["icon"].as_str().unwrap_or("");
                        let icon_str = if !icon.is_empty() {
                            format!(
                                " ![Icon](https://openweathermap.org/img/wn/{}@2x.png)",
                                icon
                            )
                        } else {
                            String::new()
                        };

                        result.push_str(&format!(
                            "- `{}`: {:.1}°{}, {}, ☔ {:.0}%{}\n",
                            time, temp, temp_unit, weather, rain_prob, icon_str
                        ));
                    }
                }
            }

            if !found {
                result.push_str(
                    "(No data found for this date. Note: Forecast is only for next 5 days)\n",
                );
            }
        } else {
            // Summarize by day (noon forecast or max temp)
            // Simplified approach: Group by day and show noon forecast + daily summary
            use std::collections::BTreeMap;

            // Map keyed by date string (YYYY-MM-DD) -> value is vector of items
            let mut daily_items: BTreeMap<String, Vec<&serde_json::Value>> = BTreeMap::new();

            for item in list {
                if let Some(dt_txt) = item["dt_txt"].as_str() {
                    let date = &dt_txt[0..10];
                    daily_items.entry(date.to_string()).or_default().push(item);
                }
            }

            for (date, items) in daily_items {
                // Find min/max temp for the day
                let mut min_temp = f64::MAX;
                let mut max_temp = f64::MIN;
                let mut descriptions = std::collections::HashSet::new();
                let mut rain_prob_max = 0.0;

                for item in &items {
                    if let Some(t) = item["main"]["temp_min"].as_f64() {
                        if t < min_temp {
                            min_temp = t;
                        }
                    }
                    if let Some(t) = item["main"]["temp_max"].as_f64() {
                        if t > max_temp {
                            max_temp = t;
                        }
                    }
                    if let Some(w) = item["weather"][0]["description"].as_str() {
                        descriptions.insert(w);
                    }
                    if let Some(pop) = item["pop"].as_f64() {
                        if pop > rain_prob_max {
                            rain_prob_max = pop;
                        }
                    }
                }

                // Pick a representative weather description (most common or just first few)
                let weather_desc = descriptions
                    .into_iter()
                    .take(2)
                    .collect::<Vec<_>>()
                    .join(", ");

                // Get a representative icon (from the middle of the day ~12:00 if possible, or just the first)
                // Simple heuristic: take the icon from the item closest to 12:00
                let mut best_icon = "";
                for item in &items {
                    if let Some(dt_txt) = item["dt_txt"].as_str() {
                        if dt_txt.contains("12:00:00") {
                            best_icon = item["weather"][0]["icon"].as_str().unwrap_or("");
                            break;
                        }
                    }
                }
                if best_icon.is_empty() && !items.is_empty() {
                    best_icon = items[0]["weather"][0]["icon"].as_str().unwrap_or("");
                }

                let icon_str = if !best_icon.is_empty() {
                    format!(
                        " ![Icon](https://openweathermap.org/img/wn/{}@2x.png)",
                        best_icon
                    )
                } else {
                    String::new()
                };

                // Parse date to simpler format (e.g., Weekday) if possible, but YYYY-MM-DD is fine for now
                result.push_str(&format!(
                    "**{}**: High {:.1}°{} / Low {:.1}°{}, {}, ☔ {:.0}% chance{}\n",
                    date,
                    max_temp,
                    temp_unit,
                    min_temp,
                    temp_unit,
                    weather_desc.to_titlecase(),
                    rain_prob_max * 100.0,
                    icon_str
                ));
            }
        }

        Ok(result)
    }
}

#[async_trait]
impl AgentTool for ForecastTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "weather_forecast",
            "description": "Get 5-day weather forecast for a location. Use this to answer questions about future weather (tomorrow, next Friday, etc.). Returns 3-hour intervals. You can filter for a specific date or get a summary.",
            "parameters": {
                "type": "object",
                "properties": {
                    "city": {
                        "type": "string",
                        "description": "Name of the city."
                    },
                    "latitude": {
                        "type": "number",
                        "description": "Latitude. If provided, skips geocoding."
                    },
                    "longitude": {
                        "type": "number",
                        "description": "Longitude. If provided, skips geocoding."
                    },
                    "state": {
                        "type": "string",
                        "description": "State code (2-letter)."
                    },
                    "country": {
                        "type": "string",
                        "description": "Country code (2-letter)."
                    },
                    "target_date": {
                        "type": "string",
                        "description": "Optional specific date to get forecast for (format: YYYY-MM-DD). If omitted, returns 5-day summary."
                    },
                    "units": {
                        "type": "string",
                        "description": "Units (standard, metric, imperial). Default: metric.",
                        "default": "metric",
                        "enum": ["standard", "metric", "imperial"]
                    }
                },
                "required": []
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse forecast arguments")?;

        let units = args
            .get("units")
            .and_then(|v| v.as_str())
            .unwrap_or("metric");
        let city = args.get("city").and_then(|v| v.as_str());
        let state = args.get("state").and_then(|v| v.as_str());
        let country = args.get("country").and_then(|v| v.as_str());
        let lat = args.get("latitude").and_then(|v| v.as_f64());
        let lon = args.get("longitude").and_then(|v| v.as_f64());
        let mut target_date = args
            .get("target_date")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Handle "tomorrow" or "today" relative dates
        if let Some(date_str) = &target_date {
            let now = Utc::now();
            let date_lower = date_str.to_lowercase();

            if date_lower.contains("tomorrow") {
                let tomorrow = now + chrono::Duration::days(1);
                target_date = Some(tomorrow.format("%Y-%m-%d").to_string());
            } else if date_lower.contains("today") {
                target_date = Some(now.format("%Y-%m-%d").to_string());
            }
        }

        // Validate units
        if !["standard", "metric", "imperial"].contains(&units) {
            return Err(anyhow::anyhow!("Invalid units"));
        }

        // Validate location
        if city.is_none() && (lat.is_none() || lon.is_none()) {
            return Err(anyhow::anyhow!("Either city or lat/lon required"));
        }

        let forecast_data = self
            .fetch_forecast_data(city, state, country, lat, lon, units)
            .await?;
        let result =
            self.format_forecast_response(&forecast_data, target_date.as_deref(), units)?;

        Ok(ToolCallResult {
            tool_name: "weather_forecast".to_string(),
            result,
        })
    }

    fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weather_metadata() {
        let tool = WeatherTool::new();
        let metadata = tool.metadata();
        assert_eq!(metadata.id, "weather_current");
        assert_eq!(metadata.category, ToolCategory::Utility);
        assert_eq!(metadata.tool_type, ToolType::Weather);
    }

    #[test]
    fn test_weather_function_definition() {
        let tool = WeatherTool::new();
        let def = tool.get_function_definition();
        assert_eq!(def["name"], "weather_current");
        assert!(def["parameters"]["properties"].get("city").is_some());
    }

    #[test]
    fn test_forecast_metadata() {
        let tool = ForecastTool::new();
        let metadata = tool.metadata();
        assert_eq!(metadata.id, "weather_forecast");
        assert_eq!(metadata.category, ToolCategory::Utility);
        assert_eq!(metadata.tool_type, ToolType::Weather);
    }

    #[test]
    fn test_weather_availability() {
        let tool = WeatherTool::new();
        // If OPENWEATHER_API_KEY is not set, is_available should be false
        if std::env::var("OPENWEATHER_API_KEY").is_err() {
            assert!(!tool.is_available());
        }
    }
}
