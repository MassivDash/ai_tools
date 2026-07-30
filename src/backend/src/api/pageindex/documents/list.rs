use crate::api::pageindex::storage::PageIndexStorage;
use actix_web::{get, web, HttpResponse, Result as ActixResult};
use std::sync::Arc;

#[get("/api/pageindex/documents")]
pub async fn list_documents(
    storage: web::Data<Arc<PageIndexStorage>>,
) -> ActixResult<HttpResponse> {
    match storage.list_documents().await {
        Ok(documents) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "documents": documents
        }))),
        Err(e) => {
            println!("⚠️ PageIndex: failed to list documents: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": e.to_string()
            })))
        }
    }
}
