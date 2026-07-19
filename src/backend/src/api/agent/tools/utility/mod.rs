pub mod ask_human;
pub mod email;
pub mod google_calendar;
pub mod google_calendar_read;
pub mod google_contacts;
pub mod google_docs;
pub mod google_drive;
pub mod google_gmail;
pub mod google_gmail_read;
pub mod google_oauth;
pub mod google_places;
pub mod google_sheets;
pub mod google_tasks;
pub mod google_youtube;
pub mod weather;

use crate::api::agent::core::types::{AgentConfig, ToolType};
use crate::api::agent::tools::framework::registry::ToolRegistry;
use crate::api::agent::tools::utility::ask_human::AskHumanTool;
use crate::api::agent::tools::utility::email::EmailTool;
use crate::api::agent::tools::utility::google_calendar::GoogleCalendarTool;
use crate::api::agent::tools::utility::google_calendar_read::GoogleCalendarReadTool;
use crate::api::agent::tools::utility::google_contacts::GoogleContactsReadTool;
use crate::api::agent::tools::utility::google_docs::{GoogleDocsReadTool, GoogleDocsWriteTool};
use crate::api::agent::tools::utility::google_drive::{GoogleDriveReadTool, GoogleDriveSearchTool};
use crate::api::agent::tools::utility::google_gmail::GoogleGmailTool;
use crate::api::agent::tools::utility::google_gmail_read::GoogleGmailReadTool;
use crate::api::agent::tools::utility::google_oauth::GoogleOAuthProvider;
use crate::api::agent::tools::utility::google_sheets::{
    GoogleSheetsReadTool, GoogleSheetsWriteTool,
};
use crate::api::agent::tools::utility::google_tasks::{GoogleTasksReadTool, GoogleTasksWriteTool};
use crate::api::agent::tools::utility::google_youtube::GoogleYouTubeReadTool;
use crate::api::agent::tools::utility::weather::{ForecastTool, WeatherTool};
use std::sync::Arc;

pub fn register(registry: &mut ToolRegistry, config: &AgentConfig) {
    if config.enabled_tools.contains(&ToolType::AskHuman) {
        let ask_human_tool = AskHumanTool::new();
        if let Err(e) = registry.register(Arc::new(ask_human_tool)) {
            println!("⚠️ Failed to register Ask Human tool: {}", e);
        }
    }

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
        || config.enabled_tools.contains(&ToolType::GoogleCalendarRead)
        || config.enabled_tools.contains(&ToolType::GoogleDriveSearch)
        || config.enabled_tools.contains(&ToolType::GoogleDriveRead)
        || config.enabled_tools.contains(&ToolType::GoogleDocsRead)
        || config.enabled_tools.contains(&ToolType::GoogleDocsWrite)
        || config.enabled_tools.contains(&ToolType::GoogleSheetsRead)
        || config.enabled_tools.contains(&ToolType::GoogleSheetsWrite)
        || config.enabled_tools.contains(&ToolType::GoogleTasksRead)
        || config.enabled_tools.contains(&ToolType::GoogleTasksWrite)
        || config.enabled_tools.contains(&ToolType::GoogleContactsRead)
        || config.enabled_tools.contains(&ToolType::GoogleYouTubeRead);

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

                if config.enabled_tools.contains(&ToolType::GoogleDriveSearch) {
                    let tool = GoogleDriveSearchTool::new(Arc::clone(&oauth_arc));
                    if let Err(e) = registry.register(Arc::new(tool)) {
                        println!("⚠️ Failed to register Google Drive Search tool: {}", e);
                    }
                }

                if config.enabled_tools.contains(&ToolType::GoogleDriveRead) {
                    let tool = GoogleDriveReadTool::new(Arc::clone(&oauth_arc));
                    if let Err(e) = registry.register(Arc::new(tool)) {
                        println!("⚠️ Failed to register Google Drive Read tool: {}", e);
                    }
                }

                if config.enabled_tools.contains(&ToolType::GoogleDocsRead) {
                    let tool = GoogleDocsReadTool::new(Arc::clone(&oauth_arc));
                    if let Err(e) = registry.register(Arc::new(tool)) {
                        println!("⚠️ Failed to register Google Docs Read tool: {}", e);
                    }
                }

                if config.enabled_tools.contains(&ToolType::GoogleDocsWrite) {
                    let tool = GoogleDocsWriteTool::new(Arc::clone(&oauth_arc));
                    if let Err(e) = registry.register(Arc::new(tool)) {
                        println!("⚠️ Failed to register Google Docs Write tool: {}", e);
                    }
                }

                if config.enabled_tools.contains(&ToolType::GoogleSheetsRead) {
                    let tool = GoogleSheetsReadTool::new(Arc::clone(&oauth_arc));
                    if let Err(e) = registry.register(Arc::new(tool)) {
                        println!("⚠️ Failed to register Google Sheets Read tool: {}", e);
                    }
                }

                if config.enabled_tools.contains(&ToolType::GoogleSheetsWrite) {
                    let tool = GoogleSheetsWriteTool::new(Arc::clone(&oauth_arc));
                    if let Err(e) = registry.register(Arc::new(tool)) {
                        println!("⚠️ Failed to register Google Sheets Write tool: {}", e);
                    }
                }

                if config.enabled_tools.contains(&ToolType::GoogleTasksRead) {
                    let tool = GoogleTasksReadTool::new(Arc::clone(&oauth_arc));
                    if let Err(e) = registry.register(Arc::new(tool)) {
                        println!("⚠️ Failed to register Google Tasks Read tool: {}", e);
                    }
                }

                if config.enabled_tools.contains(&ToolType::GoogleTasksWrite) {
                    let tool = GoogleTasksWriteTool::new(Arc::clone(&oauth_arc));
                    if let Err(e) = registry.register(Arc::new(tool)) {
                        println!("⚠️ Failed to register Google Tasks Write tool: {}", e);
                    }
                }

                if config.enabled_tools.contains(&ToolType::GoogleContactsRead) {
                    let tool = GoogleContactsReadTool::new(Arc::clone(&oauth_arc));
                    if let Err(e) = registry.register(Arc::new(tool)) {
                        println!("⚠️ Failed to register Google Contacts Read tool: {}", e);
                    }
                }

                if config.enabled_tools.contains(&ToolType::GoogleYouTubeRead) {
                    let tool = GoogleYouTubeReadTool::new(Arc::clone(&oauth_arc));
                    if let Err(e) = registry.register(Arc::new(tool)) {
                        println!("⚠️ Failed to register Google YouTube Read tool: {}", e);
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

    if config.enabled_tools.contains(&ToolType::GooglePlacesSearch) {
        let places_tool =
            crate::api::agent::tools::utility::google_places::GooglePlacesSearchTool::new();
        if let Err(e) = registry.register(Arc::new(places_tool)) {
            println!("⚠️ Failed to register Google Places Search tool: {}", e);
        }
    }
}
