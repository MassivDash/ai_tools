use crate::api::agent::types::{
    AgentConfig, AgentConfigRequest, AgentConfigResponse, AgentStatusResponse, ToolType,
};
use actix_web::{get, post, web, HttpResponse, Result as ActixResult};
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
