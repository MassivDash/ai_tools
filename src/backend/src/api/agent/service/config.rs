use crate::api::agent::core::types::{
    AgentConfig, AgentConfigRequest, AgentConfigResponse, AgentStatusResponse, ModelCapabilities,
    ModelPropsResponse, ToolType,
};
use actix_web::{get, post, web, HttpResponse, Result as ActixResult};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Shared state for agent configuration
pub type AgentConfigHandle = Arc<Mutex<AgentConfig>>;

/// Get agent status and configuration
#[get("/api/agent/status")]
pub async fn get_agent_status(
    agent_config: web::Data<AgentConfigHandle>,
) -> ActixResult<HttpResponse> {
    let config = agent_config.lock().unwrap().clone();
    Ok(HttpResponse::Ok().json(AgentStatusResponse {
        active: true, // Agent is always "active" - it's just a service
        config,
    }))
}

/// Get agent configuration
#[get("/api/agent/config")]
pub async fn get_agent_config(
    agent_config: web::Data<AgentConfigHandle>,
) -> ActixResult<HttpResponse> {
    let config = agent_config.lock().unwrap().clone();
    Ok(HttpResponse::Ok().json(config))
}

/// Update agent configuration
#[post("/api/agent/config")]
pub async fn post_agent_config(
    req: web::Json<AgentConfigRequest>,
    agent_config: web::Data<AgentConfigHandle>,
) -> ActixResult<HttpResponse> {
    let mut config_guard = agent_config.lock().unwrap();

    // Update enabled_tools if provided
    if let Some(enabled_tools) = req.enabled_tools.clone() {
        config_guard.enabled_tools = enabled_tools;
    }

    // Update debug logging if provided
    if let Some(debug_logging) = req.debug_logging {
        config_guard.debug_logging = debug_logging;
    }

    println!("✅ Agent configuration updated: {:?}", config_guard);

    Ok(HttpResponse::Ok().json(AgentConfigResponse {
        success: true,
        message: "Agent configuration updated successfully".to_string(),
    }))
}

use crate::api::agent::tools::framework::agent_tool::ToolCategory;

/// Tool metadata for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub id: String,
    pub name: String,
    pub tool_type: ToolType,
    pub description: String,
    pub category: ToolCategory,
    pub icon: String, // Material Icon name
}

/// Get list of all available tools
/// This returns all tools that are properly configured and available for use
#[get("/api/agent/tools")]
pub async fn get_available_tools() -> ActixResult<HttpResponse> {
    use crate::api::agent::core::types::ToolType;
    use crate::api::agent::tools::{self, framework::registry::ToolRegistry};

    // Create a temporary registry to discover all available tools
    let mut tool_registry = ToolRegistry::new();

    // Create a configuration that enables ALL known tools
    // We want to list everything that is available on the system
    let all_tools_config = AgentConfig {
        enabled_tools: vec![
            ToolType::ChromaDB,
            ToolType::PageIndex,
            ToolType::WebsiteCheck,
            ToolType::Weather,
            ToolType::WeatherForecast,
            ToolType::Currency,
            ToolType::Stock,
            ToolType::GitHubPublic,
            ToolType::GitHubAuthenticated,
            ToolType::Crypto,
            ToolType::GoogleBooks,
            ToolType::Email,
            ToolType::GoogleGmail,
            ToolType::GoogleCalendar,
            ToolType::GoogleGmailRead,
            ToolType::GoogleCalendarRead,
            ToolType::GoogleDriveSearch,
            ToolType::GoogleDriveRead,
            ToolType::GoogleDocsRead,
            ToolType::GoogleDocsWrite,
            ToolType::GoogleSheetsRead,
            ToolType::GoogleSheetsWrite,
            ToolType::GoogleTasksRead,
            ToolType::GoogleTasksWrite,
            ToolType::GoogleContactsRead,
            ToolType::GoogleYouTubeRead,
            ToolType::GooglePlacesSearch,
            ToolType::BlueskyPost,
            ToolType::FacebookPost,
            ToolType::FacebookPostsRead,
            ToolType::FacebookCommentsRead,
            ToolType::FacebookMessagesRead,
            ToolType::FacebookMessageSend,
            ToolType::FacebookBusinessPagesRead,
            ToolType::AskHuman,
            ToolType::SystemCommand,
        ],
        debug_logging: false,
    };

    // Context with dummy value for ChromaDB address
    // This allows ChromaDB tool to attempt registration (it might fail if it checks connection)
    let context = tools::RegisterContext {
        chroma_address: Some("http://localhost:8000"),
        available_collections: &[],
        available_page_indexes: &[],
    };

    // Register all tools
    // Note: This will only register tools that return true for is_available()
    // e.g., Weather tool will only appear if API key is set
    tools::register_all(&mut tool_registry, &all_tools_config, &context);

    // Extract metadata from registered tools
    let tools_info: Vec<ToolInfo> = tool_registry
        .get_all_tools()
        .iter()
        .map(|tool| {
            let meta = tool.metadata();
            ToolInfo {
                id: meta.id.clone(),
                name: meta.name.clone(),
                tool_type: meta.tool_type.clone(),
                description: meta.description.clone(),
                category: meta.category,
                icon: meta.category.icon_name().to_string(),
            }
        })
        .collect();

    Ok(HttpResponse::Ok().json(tools_info))
}

/// Get model capabilities from llama server /props endpoint
#[get("/api/agent/model-capabilities")]
pub async fn get_model_capabilities(
    llama_config: web::Data<Arc<Mutex<crate::api::llama_server::types::Config>>>,
) -> ActixResult<HttpResponse> {
    let client = Client::new();

    let (mut host, port) = {
        let config = llama_config.lock().unwrap();
        (
            config
                .host
                .clone()
                .unwrap_or_else(|| "localhost".to_string()),
            config.port.unwrap_or(8080),
        )
    };

    if host == "0.0.0.0" {
        host = "localhost".to_string();
    }

    let llama_url = format!("http://{}:{}/props", host, port);

    match client.get(llama_url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let json_text = response.text().await.unwrap_or_default();
                match serde_json::from_str::<ModelPropsResponse>(&json_text) {
                    Ok(props) => {
                        println!(
                            "📊 Model capabilities: vision={}, audio={}",
                            props.modalities.vision, props.modalities.audio
                        );
                        Ok(HttpResponse::Ok().json(props.modalities))
                    }
                    Err(e) => {
                        println!("⚠️ Failed to parse model props: {}", e);
                        // Return default capabilities if parsing fails
                        Ok(HttpResponse::Ok().json(ModelCapabilities {
                            vision: false,
                            audio: false,
                        }))
                    }
                }
            } else {
                println!("⚠️ Llama server returned error: {}", response.status());
                Ok(HttpResponse::Ok().json(ModelCapabilities {
                    vision: false,
                    audio: false,
                }))
            }
        }
        Err(e) => {
            println!("⚠️ Failed to connect to llama server: {}", e);
            // Return default capabilities if connection fails
            Ok(HttpResponse::Ok().json(ModelCapabilities {
                vision: false,
                audio: false,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    fn handle(config: AgentConfig) -> AgentConfigHandle {
        Arc::new(Mutex::new(config))
    }

    #[actix_web::test]
    async fn test_get_agent_status_reports_the_current_config() {
        let config = handle(AgentConfig {
            enabled_tools: vec![ToolType::Weather, ToolType::Stock],
            debug_logging: true,
        });

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .service(get_agent_status),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/agent/status")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: AgentStatusResponse = test::read_body_json(resp).await;
        assert!(
            body.active,
            "the agent service always reports itself active"
        );
        assert!(body.config.debug_logging);
        assert_eq!(
            body.config.enabled_tools,
            vec![ToolType::Weather, ToolType::Stock]
        );
    }

    #[actix_web::test]
    async fn test_get_agent_config_returns_the_defaults() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(handle(AgentConfig::default())))
                .service(get_agent_config),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/agent/config")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: AgentConfig = test::read_body_json(resp).await;
        assert!(body.enabled_tools.is_empty());
        assert!(!body.debug_logging);
    }

    #[actix_web::test]
    async fn test_post_agent_config_updates_both_fields() {
        let config = handle(AgentConfig::default());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config.clone()))
                .service(post_agent_config),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/agent/config")
            .set_json(serde_json::json!({
                "enabled_tools": ["weather", "stock"],
                "debug_logging": true
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: AgentConfigResponse = test::read_body_json(resp).await;
        assert!(body.success);
        assert_eq!(body.message, "Agent configuration updated successfully");

        let stored = config.lock().unwrap().clone();
        assert_eq!(
            stored.enabled_tools,
            vec![ToolType::Weather, ToolType::Stock]
        );
        assert!(stored.debug_logging);
    }

    #[actix_web::test]
    async fn test_post_agent_config_leaves_omitted_fields_alone() {
        let config = handle(AgentConfig {
            enabled_tools: vec![ToolType::Weather],
            debug_logging: true,
        });

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config.clone()))
                .service(post_agent_config),
        )
        .await;

        // An empty body must not clear the existing configuration
        let req = test::TestRequest::post()
            .uri("/api/agent/config")
            .set_json(serde_json::json!({}))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status().as_u16(), 200);
        {
            let stored = config.lock().unwrap();
            assert_eq!(stored.enabled_tools, vec![ToolType::Weather]);
            assert!(stored.debug_logging);
        }

        // Only debug_logging is changed here
        let req = test::TestRequest::post()
            .uri("/api/agent/config")
            .set_json(serde_json::json!({ "debug_logging": false }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status().as_u16(), 200);
        {
            let stored = config.lock().unwrap();
            assert_eq!(stored.enabled_tools, vec![ToolType::Weather]);
            assert!(!stored.debug_logging);
        }

        // An explicitly empty tool list does clear the tools
        let req = test::TestRequest::post()
            .uri("/api/agent/config")
            .set_json(serde_json::json!({ "enabled_tools": [] }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status().as_u16(), 200);
        assert!(config.lock().unwrap().enabled_tools.is_empty());
    }

    #[actix_web::test]
    async fn test_post_agent_config_rejects_an_unknown_tool_type() {
        let config = handle(AgentConfig::default());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config.clone()))
                .service(post_agent_config),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/agent/config")
            .set_json(serde_json::json!({ "enabled_tools": ["not_a_tool"] }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 400);
        assert!(config.lock().unwrap().enabled_tools.is_empty());
    }

    #[actix_web::test]
    async fn test_get_available_tools_describes_every_registered_tool() {
        let app = test::init_service(App::new().service(get_available_tools)).await;

        let req = test::TestRequest::get()
            .uri("/api/agent/tools")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let tools: Vec<ToolInfo> = test::read_body_json(resp).await;

        // Which tools register depends on the credentials present in the
        // environment, but `ask_human` needs none and is always there.
        let ids: Vec<&str> = tools.iter().map(|t| t.id.as_str()).collect();
        assert!(
            ids.contains(&"ask_human"),
            "ask_human should always be available, got {:?}",
            ids
        );
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(
            names.contains(&"Website Reader"),
            "the credential-free Website Reader should be available, got {:?}",
            names
        );

        for tool in &tools {
            assert!(!tool.id.is_empty(), "tool without an id: {:?}", tool);
            assert!(!tool.name.is_empty(), "tool without a name: {:?}", tool);
            assert!(
                !tool.description.is_empty(),
                "tool without a description: {:?}",
                tool
            );
            assert_eq!(
                tool.icon,
                tool.category.icon_name(),
                "icon should be derived from the category for {}",
                tool.id
            );
        }

        // Ids are unique - the registry is keyed by them
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "duplicate tool ids in {:?}", ids);
    }

    /// The happy path of `get_model_capabilities` needs a live llama server, but
    /// the unreachable-server fallback (and the `0.0.0.0` host rewrite that
    /// precedes the request) can be checked without one.
    #[actix_web::test]
    async fn test_get_model_capabilities_defaults_when_the_server_is_unreachable() {
        let llama_config = Arc::new(Mutex::new(crate::api::llama_server::types::Config {
            host: Some("0.0.0.0".to_string()),
            port: Some(1),
            ..Default::default()
        }));

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(llama_config))
                .service(get_model_capabilities),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/agent/model-capabilities")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: ModelCapabilities = test::read_body_json(resp).await;
        assert!(!body.vision);
        assert!(!body.audio);
    }
}
