use crate::api::model_notes::sqlite_storage::ModelNotesStorage;
use crate::api::model_notes::types::ModelNoteResponse;
use actix_web::{get, web, HttpResponse, Result as ActixResult};
use std::sync::Arc;

#[get("/api/model-notes/default/{platform}")]
pub async fn get_default_model(
    path: web::Path<String>,
    storage: web::Data<Arc<ModelNotesStorage>>,
) -> ActixResult<HttpResponse> {
    let platform = path.into_inner();
    println!("📋 Fetching default model for platform: {}", platform);

    // Validate platform
    if platform != "llama" && platform != "ollama" {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Invalid platform: {}. Must be 'llama' or 'ollama'", platform)
        })));
    }

    match storage.get_default_model(&platform).await {
        Ok(Some(note)) => {
            println!(
                "✅ Found default model for {}: {}",
                platform, note.model_name
            );
            Ok(HttpResponse::Ok().json(ModelNoteResponse { note }))
        }
        Ok(None) => {
            println!("ℹ️  No default model set for platform: {}", platform);
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": format!("No default model set for platform: {}", platform)
            })))
        }
        Err(e) => {
            println!("Failed to fetch default model: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to fetch default model: {}", e)
            })))
        }
    }
}
