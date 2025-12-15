use crate::api::model_notes::sqlite_storage::ModelNotesStorage;
use crate::api::model_notes::types::{ModelNote, ModelNoteRequest, ModelNoteResponse};
use actix_web::{post, web, HttpResponse, Result as ActixResult};
use std::sync::Arc;

#[post("/api/model-notes")]
pub async fn create_or_update_model_note(
    req: web::Json<ModelNoteRequest>,
    storage: web::Data<Arc<ModelNotesStorage>>,
) -> ActixResult<HttpResponse> {
    println!(
        "💾 Creating/updating model note for {}:{} (favorite: {:?})",
        req.platform, req.model_name, req.is_favorite
    );

    // Validate platform
    if req.platform != "llama" && req.platform != "ollama" {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Invalid platform: {}. Must be 'llama' or 'ollama'", req.platform)
        })));
    }

    let note = ModelNote {
        id: None,
        platform: req.platform.clone(),
        model_name: req.model_name.clone(),
        model_path: req.model_path.clone(),
        is_favorite: req.is_favorite.unwrap_or(false),
        tags: req.tags.clone().unwrap_or_default(),
        notes: req.notes.clone(),
        created_at: None,
        updated_at: None,
    };

    match storage.upsert_note(&note).await {
        Ok(saved_note) => {
            println!(
                "✅ Successfully saved model note for {}:{}",
                saved_note.platform, saved_note.model_name
            );
            Ok(HttpResponse::Ok().json(ModelNoteResponse { note: saved_note }))
        }
        Err(e) => {
            println!("❌ Failed to save model note: {}", e);
            println!(
                "   Platform: {}, Model: {}, Favorite: {}",
                note.platform, note.model_name, note.is_favorite
            );
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to save model note: {}", e)
            })))
        }
    }
}
