use crate::api::agent::tool_groups::sqlite_storage::ToolGroupsStorage;
use crate::api::agent::tool_groups::types::{CreateToolGroupRequest, ToolGroupResponse};
use actix_web::{post, web, HttpResponse, Result as ActixResult};

#[post("/api/agent/tool-groups")]
pub async fn create_tool_group(
    req: web::Json<CreateToolGroupRequest>,
    storage: web::Data<ToolGroupsStorage>,
) -> ActixResult<HttpResponse> {
    let name = req.name.trim();
    if name.is_empty() || req.tool_types.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "A group name and at least one tool are required"
        })));
    }

    match storage.create_group(name, &req.tool_types).await {
        Ok(group) => Ok(HttpResponse::Ok().json(ToolGroupResponse { group })),
        Err(e) if e.to_string().contains("UNIQUE constraint") => {
            Ok(HttpResponse::Conflict().json(serde_json::json!({
                "error": format!("A tool group named \"{}\" already exists", name)
            })))
        }
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to create tool group: {}", e)
        }))),
    }
}
