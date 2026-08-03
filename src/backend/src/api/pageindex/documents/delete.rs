use crate::api::pageindex::storage::PageIndexStorage;
use actix_web::{delete, web, HttpResponse, Result as ActixResult};
use std::sync::Arc;

#[delete("/api/pageindex/documents/{id}")]
pub async fn delete_document(
    path: web::Path<String>,
    storage: web::Data<Arc<PageIndexStorage>>,
) -> ActixResult<HttpResponse> {
    let id = path.into_inner();

    if let Err(e) = storage.delete_document(&id).await {
        println!("⚠️ PageIndex: failed to delete document '{}': {}", id, e);
        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": e.to_string()
        })));
    }

    let dir = std::path::Path::new("./public/pageindex").join(&id);
    if dir.exists() {
        if let Err(e) = tokio::fs::remove_dir_all(&dir).await {
            println!(
                "⚠️ PageIndex: failed to remove directory {:?} for document '{}': {}",
                dir, id, e
            );
        }
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({ "success": true })))
}
