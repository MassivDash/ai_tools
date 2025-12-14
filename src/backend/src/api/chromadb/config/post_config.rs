use crate::api::chromadb::config::types::{ChromaDBConfig, ConfigRequest, ConfigUpdateResponse};
use actix_web::{post, web, HttpResponse, Result as ActixResult};
use std::sync::{Arc, Mutex};

#[post("/api/chromadb/config")]
pub async fn post_chromadb_config(
    request: web::Json<ConfigRequest>,
    config: web::Data<Arc<Mutex<ChromaDBConfig>>>,
) -> ActixResult<HttpResponse> {
    println!("💾 Updating ChromaDB configuration...");

    // Validate that embedding_model is not empty
    if request.embedding_model.trim().is_empty() {
        return Ok(HttpResponse::BadRequest().json(ConfigUpdateResponse {
            success: false,
            message: "Embedding model cannot be empty".to_string(),
        }));
    }

    let mut config_guard = config.lock().unwrap();
    config_guard.embedding_model = request.embedding_model.clone();
    config_guard.query_model = request
        .query_model
        .clone()
        .unwrap_or_else(|| request.embedding_model.clone());

    println!(
        "✅ Updated config - Embedding: {}, Query: {}",
        config_guard.embedding_model, config_guard.query_model
    );

    Ok(HttpResponse::Ok().json(ConfigUpdateResponse {
        success: true,
        message: "Configuration updated successfully".to_string(),
    }))
}
