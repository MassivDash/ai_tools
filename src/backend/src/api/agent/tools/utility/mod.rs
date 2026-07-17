pub mod email;
pub mod weather;

use crate::api::agent::core::types::{AgentConfig, ToolType};
use crate::api::agent::tools::framework::registry::ToolRegistry;
use crate::api::agent::tools::utility::email::EmailTool;
use crate::api::agent::tools::utility::weather::{ForecastTool, WeatherTool};
use std::sync::Arc;

pub fn register(registry: &mut ToolRegistry, config: &AgentConfig) {
    if config.enabled_tools.contains(&ToolType::Weather) {
        let weather_tool = WeatherTool::new();
        if let Err(e) = registry.register(Arc::new(weather_tool)) {
            println!("⚠️ Failed to register Weather tool: {}", e);
        }

        let forecast_tool = ForecastTool::new();
        if let Err(e) = registry.register(Arc::new(forecast_tool)) {
            println!("⚠️ Failed to register Forecast tool: {}", e);
        }
    }

    if config.enabled_tools.contains(&ToolType::Email) {
        let email_tool = EmailTool::new();
        if let Err(e) = registry.register(Arc::new(email_tool)) {
            println!("⚠️ Failed to register Email tool: {}", e);
        }
    }
}
