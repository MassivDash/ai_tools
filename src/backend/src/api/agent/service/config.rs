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
            ToolType::WebsiteCheck,
            ToolType::Weather,
            ToolType::Currency,
            ToolType::Stock,
            ToolType::GitHubPublic,
            ToolType::GitHubAuthenticated,
            ToolType::Crypto,
            ToolType::GoogleBooks,
            ToolType::Email,
        ],
        debug_logging: false,
    };

    // Context with dummy value for ChromaDB address
    // This allows ChromaDB tool to attempt registration (it might fail if it checks connection)
    let context = tools::RegisterContext {
        chroma_address: Some("http://localhost:8000"),
        available_collections: &[],
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
