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
    use crate::api::agent::core::types::{ChromaDBToolConfig, ToolType};
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
            ToolType::GitHubPublic,
            ToolType::GitHubAuthenticated,
        ],
        // Provide dummy config for ChromaDB so it attempts registration
        // It will only succeed if the code handles it, but connection check might fail it effectively.
        chromadb: Some(ChromaDBToolConfig {
            collection: "metadata_check".to_string(),
            embedding_model: "metadata_check".to_string(),
        }),
    };

    // Context with dummy value for ChromaDB address
    // This allows ChromaDB tool to attempt registration (it might fail if it checks connection)
    let context = tools::RegisterContext {
        chroma_address: Some("http://localhost:8000"),
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
            let def = tool.get_function_definition();

            // Description comes from the function definition
            let description = def
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("No description available")
                .to_string();

            ToolInfo {
                id: meta.id.clone(),
                name: meta.name.clone(),
                tool_type: meta.tool_type.clone(),
                description,
                category: meta.category,
                icon: meta.category.icon_name().to_string(),
            }
        })
        .collect();

    Ok(HttpResponse::Ok().json(tools_info))
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
