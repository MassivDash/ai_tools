pub mod email;
pub mod google_calendar;
pub mod google_calendar_read;
pub mod google_gmail;
pub mod google_gmail_read;
pub mod google_oauth;
pub mod weather;

use crate::api::agent::core::types::{AgentConfig, ToolType};
use crate::api::agent::tools::framework::registry::ToolRegistry;
use crate::api::agent::tools::utility::email::EmailTool;
use crate::api::agent::tools::utility::google_calendar::GoogleCalendarTool;
use crate::api::agent::tools::utility::google_calendar_read::GoogleCalendarReadTool;
use crate::api::agent::tools::utility::google_gmail::GoogleGmailTool;
use crate::api::agent::tools::utility::google_gmail_read::GoogleGmailReadTool;
use crate::api::agent::tools::utility::google_oauth::GoogleOAuthProvider;
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

    let has_google_tools = config.enabled_tools.contains(&ToolType::GoogleGmail)
        || config.enabled_tools.contains(&ToolType::GoogleCalendar)
        || config.enabled_tools.contains(&ToolType::GoogleGmailRead)
        || config.enabled_tools.contains(&ToolType::GoogleCalendarRead);

    if has_google_tools {
        if GoogleOAuthProvider::is_configured() {
            if let Ok(oauth_provider) = GoogleOAuthProvider::new() {
                let oauth_arc = Arc::new(oauth_provider);

                if config.enabled_tools.contains(&ToolType::GoogleGmail) {
                    let gmail_tool = GoogleGmailTool::new(Arc::clone(&oauth_arc));
                    if let Err(e) = registry.register(Arc::new(gmail_tool)) {
                        println!("⚠️ Failed to register Google Gmail tool: {}", e);
                    }
                }

                if config.enabled_tools.contains(&ToolType::GoogleGmailRead) {
                    let gmail_read_tool = GoogleGmailReadTool::new(Arc::clone(&oauth_arc));
                    if let Err(e) = registry.register(Arc::new(gmail_read_tool)) {
                        println!("⚠️ Failed to register Google Gmail Read tool: {}", e);
                    }
                }

                if config.enabled_tools.contains(&ToolType::GoogleCalendar) {
                    let calendar_tool = GoogleCalendarTool::new(Arc::clone(&oauth_arc));
                    if let Err(e) = registry.register(Arc::new(calendar_tool)) {
                        println!("⚠️ Failed to register Google Calendar tool: {}", e);
                    }
                }

                if config.enabled_tools.contains(&ToolType::GoogleCalendarRead) {
                    let calendar_read_tool = GoogleCalendarReadTool::new(Arc::clone(&oauth_arc));
                    if let Err(e) = registry.register(Arc::new(calendar_read_tool)) {
                        println!("⚠️ Failed to register Google Calendar Read tool: {}", e);
                    }
                }
            } else {
                println!(
                    "⚠️ Google Workspace tools enabled, but failed to initialize OAuth Provider."
                );
            }
        } else {
            println!("⚠️ Google Workspace tools enabled, but missing GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET, or GOOGLE_REFRESH_TOKEN.");
        }
    }
}
