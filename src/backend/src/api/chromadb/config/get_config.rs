use crate::api::chromadb::config::types::{ChromaDBConfig, ConfigResponse};
use actix_web::{get, web, HttpResponse, Result as ActixResult};
use std::sync::{Arc, Mutex};

#[get("/api/chromadb/config")]
pub async fn get_chromadb_config(
    config: web::Data<Arc<Mutex<ChromaDBConfig>>>,
) -> ActixResult<HttpResponse> {
    println!("📋 Fetching ChromaDB configuration...");

    let config_guard = config.lock().unwrap();
    let config_response = ConfigResponse {
        embedding_model: config_guard.embedding_model.clone(),
        query_model: config_guard.query_model.clone(),
        chunk_size: config_guard.chunk_size,
        chunk_overlap: config_guard.chunk_overlap,
    };

    println!(
        "✅ Current config - Embedding: {}, Query: {}",
        config_response.embedding_model, config_response.query_model
    );

    Ok(HttpResponse::Ok().json(config_response))
}
