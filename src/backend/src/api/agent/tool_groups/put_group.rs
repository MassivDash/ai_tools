use crate::api::agent::tool_groups::sqlite_storage::ToolGroupsStorage;
use crate::api::agent::tool_groups::types::{ToolGroupResponse, UpdateToolGroupRequest};
use actix_web::{put, web, HttpResponse, Result as ActixResult};

#[put("/api/agent/tool-groups/{id}")]
pub async fn update_tool_group(
    path: web::Path<i64>,
    req: web::Json<UpdateToolGroupRequest>,
    storage: web::Data<ToolGroupsStorage>,
) -> ActixResult<HttpResponse> {
    let id = path.into_inner();
    let name = req.name.trim();
    if name.is_empty() || req.tool_types.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "A group name and at least one tool are required"
        })));
    }

    match storage.update_group(id, name, &req.tool_types).await {
        Ok(Some(group)) => Ok(HttpResponse::Ok().json(ToolGroupResponse { group })),
        Ok(None) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Tool group not found"
        }))),
        Err(e) if e.to_string().contains("UNIQUE constraint") => {
            Ok(HttpResponse::Conflict().json(serde_json::json!({
                "error": format!("A tool group named \"{}\" already exists", name)
            })))
        }
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to update tool group: {}", e)
        }))),
    }
}
