use crate::api::agent::core::types::{ToolCall, ToolCallResult};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
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
        let api_key = env::var("OPENWEATHER_API_KEY")
            .expect("OPENWEATHER_API_KEY environment variable must be set");

        Self {
            metadata: ToolMetadata {
                id: "4".to_string(), // Using next available ID after 3 (WebsiteCheck)
                name: "weather_check".to_string(),
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
                "\nWeather icon code: {} (use with OpenWeatherMap icon service)\n",
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
            "name": "weather_check",
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
