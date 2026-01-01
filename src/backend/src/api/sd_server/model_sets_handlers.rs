use actix_web::{delete, get, post, put, web, HttpResponse, Result as ActixResult};
use serde::Deserialize;
use std::sync::Arc;

use crate::api::sd_server::model_sets::SDModelSetsStorage;

#[derive(Deserialize)]
pub struct CreateModelSetRequest {
    pub name: String,
    pub diffusion_model: String,
    pub vae: Option<String>,
    pub llm: Option<String>,
    pub is_default: bool,
}

#[derive(Deserialize)]
pub struct UpdateModelSetRequest {
    pub name: String,
    pub diffusion_model: String,
    pub vae: Option<String>,
    pub llm: Option<String>,
    pub is_default: bool,
}

#[get("/api/sd-server/model-sets")]
pub async fn list_model_sets(
    storage: web::Data<Arc<SDModelSetsStorage>>,
) -> ActixResult<HttpResponse> {
    match storage.list().await {
        Ok(sets) => Ok(HttpResponse::Ok().json(sets)),
        Err(e) => {
            Ok(HttpResponse::InternalServerError().json(format!("Failed to list sets: {}", e)))
        }
    }
}

#[post("/api/sd-server/model-sets")]
pub async fn create_model_set(
    body: web::Json<CreateModelSetRequest>,
    storage: web::Data<Arc<SDModelSetsStorage>>,
) -> ActixResult<HttpResponse> {
    match storage
        .create(
            body.name.clone(),
            body.diffusion_model.clone(),
            body.vae.clone(),
            body.llm.clone(),
            body.is_default,
        )
        .await
    {
        Ok(set) => Ok(HttpResponse::Ok().json(set)),
        Err(e) => {
            Ok(HttpResponse::InternalServerError().json(format!("Failed to create set: {}", e)))
        }
    }
}

#[put("/api/sd-server/model-sets/{id}")]
pub async fn update_model_set(
    path: web::Path<i64>,
    body: web::Json<UpdateModelSetRequest>,
    storage: web::Data<Arc<SDModelSetsStorage>>,
) -> ActixResult<HttpResponse> {
    let id = path.into_inner();
    match storage
        .update(
            id,
            body.name.clone(),
            body.diffusion_model.clone(),
            body.vae.clone(),
            body.llm.clone(),
            body.is_default,
        )
        .await
    {
        Ok(_) => Ok(HttpResponse::Ok().json("Updated")),
        Err(e) => {
            Ok(HttpResponse::InternalServerError().json(format!("Failed to update set: {}", e)))
        }
    }
}

#[delete("/api/sd-server/model-sets/{id}")]
pub async fn delete_model_set(
    path: web::Path<i64>,
    storage: web::Data<Arc<SDModelSetsStorage>>,
) -> ActixResult<HttpResponse> {
    let id = path.into_inner();
    match storage.delete(id).await {
        Ok(_) => Ok(HttpResponse::Ok().json("Deleted")),
        Err(e) => {
            Ok(HttpResponse::InternalServerError().json(format!("Failed to delete set: {}", e)))
        }
    }
}
