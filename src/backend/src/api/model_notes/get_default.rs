use crate::api::model_notes::sqlite_storage::ModelNotesStorage;
use crate::api::model_notes::types::ModelNoteResponse;
use actix_web::{get, web, HttpResponse, Result as ActixResult};
use std::sync::Arc;

#[get("/api/model-notes/default/{platform}")]
pub async fn get_default_model(
    path: web::Path<String>,
    storage: web::Data<Arc<ModelNotesStorage>>,
) -> ActixResult<HttpResponse> {
    let platform = path.into_inner();
    println!("📋 Fetching default model for platform: {}", platform);

    // Validate platform
    if platform != "llama" && platform != "ollama" {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Invalid platform: {}. Must be 'llama' or 'ollama'", platform)
        })));
    }

    match storage.get_default_model(&platform).await {
        Ok(Some(note)) => {
            println!(
                "✅ Found default model for {}: {}",
                platform, note.model_name
            );
            Ok(HttpResponse::Ok().json(ModelNoteResponse { note }))
        }
        Ok(None) => {
            println!("ℹ️  No default model set for platform: {}", platform);
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": format!("No default model set for platform: {}", platform)
            })))
        }
        Err(e) => {
            println!("Failed to fetch default model: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to fetch default model: {}", e)
            })))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::model_notes::sqlite_storage::new_test_storage;
    use crate::api::model_notes::types::ModelNote;
    use actix_web::{test, App};

    fn default_note(platform: &str, model_name: &str) -> ModelNote {
        ModelNote {
            id: None,
            platform: platform.to_string(),
            model_name: model_name.to_string(),
            model_path: None,
            is_favorite: false,
            is_default: true,
            tags: Vec::new(),
            notes: None,
            created_at: None,
            updated_at: None,
        }
    }

    fn request(platform: &str) -> test::TestRequest {
        test::TestRequest::get().uri(&format!("/api/model-notes/default/{}", platform))
    }

    #[actix_web::test]
    async fn test_get_default_model_rejects_unknown_platform() {
        let (_dir, storage) = new_test_storage().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(get_default_model),
        )
        .await;

        let resp = test::call_service(&app, request("openai").to_request()).await;

        assert_eq!(resp.status().as_u16(), 400);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(
            body["error"],
            "Invalid platform: openai. Must be 'llama' or 'ollama'"
        );
    }

    #[actix_web::test]
    async fn test_get_default_model_not_found() {
        let (_dir, storage) = new_test_storage().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(get_default_model),
        )
        .await;

        let resp = test::call_service(&app, request("llama").to_request()).await;

        assert_eq!(resp.status().as_u16(), 404);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "No default model set for platform: llama");
    }

    #[actix_web::test]
    async fn test_get_default_model_returns_the_default_for_each_platform() {
        let (_dir, storage) = new_test_storage().await;
        storage
            .upsert_note(&default_note("llama", "llama-default"))
            .await
            .unwrap();
        storage
            .upsert_note(&default_note("ollama", "ollama-default"))
            .await
            .unwrap();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(get_default_model),
        )
        .await;

        let resp = test::call_service(&app, request("llama").to_request()).await;
        assert_eq!(resp.status().as_u16(), 200);
        let body: ModelNoteResponse = test::read_body_json(resp).await;
        assert_eq!(body.note.model_name, "llama-default");
        assert!(body.note.is_default);

        let resp = test::call_service(&app, request("ollama").to_request()).await;
        assert_eq!(resp.status().as_u16(), 200);
        let body: ModelNoteResponse = test::read_body_json(resp).await;
        assert_eq!(body.note.model_name, "ollama-default");
    }

    #[actix_web::test]
    async fn test_get_default_model_storage_failure_returns_500() {
        let (_dir, storage) = new_test_storage().await;
        storage.drop_table_for_tests().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(get_default_model),
        )
        .await;

        let resp = test::call_service(&app, request("llama").to_request()).await;

        assert_eq!(resp.status().as_u16(), 500);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body["error"]
            .as_str()
            .unwrap()
            .starts_with("Failed to fetch default model:"));
    }
}
