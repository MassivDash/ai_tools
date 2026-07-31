use crate::api::agent::tool_groups::sqlite_storage::ToolGroupsStorage;
use actix_web::{delete, web, HttpResponse, Result as ActixResult};

#[delete("/api/agent/tool-groups/{id}")]
pub async fn delete_tool_group(
    path: web::Path<i64>,
    storage: web::Data<ToolGroupsStorage>,
) -> ActixResult<HttpResponse> {
    let id = path.into_inner();

    match storage.delete_group(id).await {
        Ok(true) => Ok(HttpResponse::Ok().json(serde_json::json!({ "success": true }))),
        Ok(false) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Tool group not found"
        }))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to delete tool group: {}", e)
        }))),
    }
}
