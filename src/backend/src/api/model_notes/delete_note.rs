use crate::api::model_notes::sqlite_storage::ModelNotesStorage;
use actix_web::{delete, web, HttpResponse, Result as ActixResult};
use std::sync::Arc;

#[delete("/api/model-notes/{platform}/{model_name}")]
pub async fn delete_model_note(
    path: web::Path<(String, String)>,
    storage: web::Data<Arc<ModelNotesStorage>>,
) -> ActixResult<HttpResponse> {
    let (platform, model_name) = path.into_inner();
    println!("🗑️  Deleting model note for {}:{}", platform, model_name);

    match storage.delete_note(&platform, &model_name).await {
        Ok(true) => {
            println!("✅ Successfully deleted model note");
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "Model note deleted successfully"
            })))
        }
        Ok(false) => {
            println!("⚠️  Model note not found");
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "Model note not found"
            })))
        }
        Err(e) => {
            println!("❌ Failed to delete model note: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to delete model note: {}", e)
            })))
        }
    }
}
