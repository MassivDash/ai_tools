use crate::api::agent::tool_groups::sqlite_storage::ToolGroupsStorage;
use crate::api::agent::tool_groups::types::ToolGroupsResponse;
use actix_web::{get, web, HttpResponse, Result as ActixResult};

#[get("/api/agent/tool-groups")]
pub async fn get_tool_groups(storage: web::Data<ToolGroupsStorage>) -> ActixResult<HttpResponse> {
    match storage.get_all_groups().await {
        Ok(groups) => Ok(HttpResponse::Ok().json(ToolGroupsResponse { groups })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to fetch tool groups: {}", e)
        }))),
    }
}
