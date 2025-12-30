use crate::api::model_notes::sqlite_storage::ModelNotesStorage;
use crate::api::model_notes::types::ModelNotesResponse;
use actix_web::{get, web, HttpResponse, Result as ActixResult};
use std::sync::Arc;

#[get("/api/model-notes")]
pub async fn get_model_notes(
    storage: web::Data<Arc<ModelNotesStorage>>,
) -> ActixResult<HttpResponse> {
    println!("📋 Fetching all model notes...");

    match storage.get_all_notes().await {
        Ok(notes) => {
            println!("✅ Found {} model notes", notes.len());
            Ok(HttpResponse::Ok().json(ModelNotesResponse { notes }))
        }
        Err(e) => {
            println!("Failed to fetch model notes: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to fetch model notes: {}", e)
            })))
        }
    }
}
