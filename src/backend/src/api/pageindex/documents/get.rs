use crate::api::pageindex::storage::PageIndexStorage;
use crate::api::pageindex::types::PageIndexNode;
use actix_web::{get, web, HttpResponse, Result as ActixResult};
use std::sync::Arc;

#[get("/api/pageindex/documents/{id}")]
pub async fn get_document(
    path: web::Path<String>,
    storage: web::Data<Arc<PageIndexStorage>>,
) -> ActixResult<HttpResponse> {
    let id = path.into_inner();

    let document = match storage.get_document(&id).await {
        Ok(Some(doc)) => doc,
        Ok(None) => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "success": false,
                "error": format!("Document '{}' not found", id)
            })));
        }
        Err(e) => {
            println!("⚠️ PageIndex: failed to fetch document '{}': {}", id, e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": e.to_string()
            })));
        }
    };

    // While the document is still processing (or if something went wrong writing it),
    // tree.json may not exist yet - that's not an error, just an empty tree.
    let tree_path = std::path::Path::new("./public/pageindex")
        .join(&id)
        .join("tree.json");

    let tree: Vec<PageIndexNode> = if tree_path.exists() {
        match tokio::fs::read_to_string(&tree_path).await {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "document": document,
        "tree": tree
    })))
}
