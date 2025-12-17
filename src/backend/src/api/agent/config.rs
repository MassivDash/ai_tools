use crate::api::agent::types::{
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

    // Validate ChromaDB config if provided
    if req.chromadb.is_some() {
        let chromadb_config = req.chromadb.as_ref().unwrap();
        if chromadb_config.collection.trim().is_empty()
            || chromadb_config.embedding_model.trim().is_empty()
        {
            return Ok(HttpResponse::BadRequest().json(AgentConfigResponse {
                success: false,
                message: "ChromaDB configuration requires both collection and embedding_model"
                    .to_string(),
            }));
        }
    }

    // Remove ChromaDB from enabled_tools if present (it's now a separate config section)
    let mut enabled_tools = req.enabled_tools.clone();
    enabled_tools.retain(|t| *t != ToolType::ChromaDB);

    // Update configuration
    config_guard.enabled_tools = enabled_tools;
    config_guard.chromadb = req.chromadb.clone();

    println!("✅ Agent configuration updated: {:?}", config_guard);

    Ok(HttpResponse::Ok().json(AgentConfigResponse {
        success: true,
        message: "Agent configuration updated successfully".to_string(),
    }))
}

use crate::api::agent::tools::agent_tool::ToolCategory;

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
/// This returns all possible tools (not just enabled ones) so the frontend can show them
#[get("/api/agent/tools")]
pub async fn get_available_tools() -> ActixResult<HttpResponse> {
    use crate::api::agent::tools::agent_tool::ToolCategory;
    use crate::api::agent::types::ToolType;

    // Return all available tools with their metadata
    // This includes all tools that can be enabled, not just currently enabled ones
    let tools = vec![
        ToolInfo {
            id: "2".to_string(),
            name: "financial sql query".to_string(),
            tool_type: ToolType::FinancialData,
            description: "Get financial data including recent purchases and transactions"
                .to_string(),
            category: ToolCategory::Financial,
            icon: ToolCategory::Financial.icon_name().to_string(),
        },
        ToolInfo {
            id: "3".to_string(),
            name: "website check".to_string(),
            tool_type: ToolType::WebsiteCheck,
            description:
                "Fetch a website URL, convert it to markdown, and provide the content for analysis"
                    .to_string(),
            category: ToolCategory::Web,
            icon: ToolCategory::Web.icon_name().to_string(),
        },
        ToolInfo {
            id: "4".to_string(),
            name: "weather".to_string(),
            tool_type: ToolType::Weather,
            description: "Get the current weather for a given city".to_string(),
            category: ToolCategory::Utility,
            icon: ToolCategory::Utility.icon_name().to_string(),
        },
        // Note: ChromaDB is special and only appears when configured, so we don't include it here
        // The frontend should handle ChromaDB separately based on configuration
    ];

    Ok(HttpResponse::Ok().json(tools))
}

/// Get model capabilities from llama server /props endpoint
#[get("/api/agent/model-capabilities")]
pub async fn get_model_capabilities() -> ActixResult<HttpResponse> {
    let client = Client::new();
    let llama_url = "http://localhost:8080/props";

    match client.get(llama_url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<ModelPropsResponse>().await {
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
                // Return default capabilities if server is not available
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
