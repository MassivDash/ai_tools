use crate::api::agent::testing::storage::TestingStorage;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateSuiteRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateSuiteRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct AddQuestionRequest {
    pub content: String,
}

#[derive(Deserialize)]
pub struct UpdateQuestionRequest {
    pub content: String,
}

#[get("/suites")]
pub async fn get_suites(storage: web::Data<TestingStorage>) -> impl Responder {
    match storage.get_suites().await {
        Ok(suites) => HttpResponse::Ok().json(suites),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": e.to_string()
        })),
    }
}

#[post("/suites")]
pub async fn create_suite(
    storage: web::Data<TestingStorage>,
    req: web::Json<CreateSuiteRequest>,
) -> impl Responder {
    match storage
        .create_suite(req.name.clone(), req.description.clone())
        .await
    {
        Ok(suite) => HttpResponse::Ok().json(suite),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": e.to_string()
        })),
    }
}

#[put("/suites/{id}")]
pub async fn update_suite(
    storage: web::Data<TestingStorage>,
    id: web::Path<String>,
    req: web::Json<UpdateSuiteRequest>,
) -> impl Responder {
    match storage
        .update_suite(&id, req.name.clone(), req.description.clone())
        .await
    {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "success": true })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": e.to_string()
        })),
    }
}

#[delete("/suites/{id}")]
pub async fn delete_suite(
    storage: web::Data<TestingStorage>,
    id: web::Path<String>,
) -> impl Responder {
    match storage.delete_suite(&id).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "success": true })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": e.to_string()
        })),
    }
}

#[get("/suites/{id}/questions")]
pub async fn get_questions(
    storage: web::Data<TestingStorage>,
    id: web::Path<String>,
) -> impl Responder {
    match storage.get_questions(&id).await {
        Ok(questions) => HttpResponse::Ok().json(questions),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": e.to_string()
        })),
    }
}

#[post("/suites/{id}/questions")]
pub async fn add_question(
    storage: web::Data<TestingStorage>,
    id: web::Path<String>,
    req: web::Json<AddQuestionRequest>,
) -> impl Responder {
    match storage.add_question(&id, req.content.clone()).await {
        Ok(question) => HttpResponse::Ok().json(question),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": e.to_string()
        })),
    }
}

#[put("/questions/{id}")]
pub async fn update_question(
    storage: web::Data<TestingStorage>,
    id: web::Path<i64>,
    req: web::Json<UpdateQuestionRequest>,
) -> impl Responder {
    match storage.update_question(*id, req.content.clone()).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "success": true })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": e.to_string()
        })),
    }
}

#[delete("/questions/{id}")]
pub async fn delete_question(
    storage: web::Data<TestingStorage>,
    id: web::Path<i64>,
) -> impl Responder {
    match storage.delete_question(*id).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "success": true })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": e.to_string()
        })),
    }
}
