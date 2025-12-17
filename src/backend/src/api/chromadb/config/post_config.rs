use crate::api::chromadb::config::types::{ChromaDBConfig, ConfigRequest, ConfigUpdateResponse};
use crate::api::default_configs::{ChromaDBDefaultConfig, DefaultConfigsStorage};
use actix_web::{post, web, HttpResponse, Result as ActixResult};
use std::sync::{Arc, Mutex};

#[post("/api/chromadb/config")]
pub async fn post_chromadb_config(
    request: web::Json<ConfigRequest>,
    config: web::Data<Arc<Mutex<ChromaDBConfig>>>,
    default_configs: web::Data<Arc<DefaultConfigsStorage>>,
) -> ActixResult<HttpResponse> {
    println!("💾 Updating ChromaDB configuration...");

    // Validate that embedding_model is not empty
    if request.embedding_model.trim().is_empty() {
        return Ok(HttpResponse::BadRequest().json(ConfigUpdateResponse {
            success: false,
            message: "Embedding model cannot be empty".to_string(),
        }));
    }

    let embedding_model = request.embedding_model.clone();
    let query_model = request
        .query_model
        .clone()
        .unwrap_or_else(|| embedding_model.clone());

    // Update config (drop lock before await)
    {
        let mut config_guard = config.lock().unwrap();
        config_guard.embedding_model = embedding_model.clone();
        config_guard.query_model = query_model.clone();
        println!(
            "✅ Updated config - Embedding: {}, Query: {}",
            config_guard.embedding_model, config_guard.query_model
        );
    } // Drop lock here

    // Save as default config (embedding_model is primary for chromadb) - lock is dropped
    if let Err(e) = default_configs
        .set_chromadb_default(&ChromaDBDefaultConfig {
            embedding_model: embedding_model.clone(),
        })
        .await
    {
        println!("⚠️  Failed to save chromadb default config: {}", e);
    } else {
        println!("✅ Saved chromadb default config");
    }

    Ok(HttpResponse::Ok().json(ConfigUpdateResponse {
        success: true,
        message: "Configuration updated successfully".to_string(),
    }))
}
