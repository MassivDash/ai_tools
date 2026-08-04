use crate::api::model_notes::sqlite_storage::ModelNotesStorage;
use crate::api::model_notes::types::{ModelNote, ModelNoteRequest, ModelNoteResponse};
use actix_web::{post, web, HttpResponse, Result as ActixResult};
use std::sync::Arc;

#[post("/api/model-notes")]
pub async fn create_or_update_model_note(
    req: web::Json<ModelNoteRequest>,
    storage: web::Data<Arc<ModelNotesStorage>>,
) -> ActixResult<HttpResponse> {
    println!(
        "💾 Creating/updating model note for {}:{} (favorite: {:?})",
        req.platform, req.model_name, req.is_favorite
    );

    // Validate platform
    if req.platform != "llama" && req.platform != "ollama" {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Invalid platform: {}. Must be 'llama' or 'ollama'", req.platform)
        })));
    }

    // For default models, don't store the path - just the name
    // Backend will handle downloading/caching automatically
    let model_path = if req.is_default.unwrap_or(false) {
        None
    } else {
        req.model_path.clone()
    };

    let note = ModelNote {
        id: None,
        platform: req.platform.clone(),
        model_name: req.model_name.clone(),
        model_path,
        is_favorite: req.is_favorite.unwrap_or(false),
        is_default: req.is_default.unwrap_or(false),
        tags: req.tags.clone().unwrap_or_default(),
        notes: req.notes.clone(),
        created_at: None,
        updated_at: None,
    };

    match storage.upsert_note(&note).await {
        Ok(saved_note) => {
            println!(
                "✅ Successfully saved model note for {}:{}",
                saved_note.platform, saved_note.model_name
            );
            Ok(HttpResponse::Ok().json(ModelNoteResponse { note: saved_note }))
        }
        Err(e) => {
            println!("Failed to save model note: {}", e);
            println!(
                "   Platform: {}, Model: {}, Favorite: {}",
                note.platform, note.model_name, note.is_favorite
            );
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to save model note: {}", e)
            })))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::model_notes::sqlite_storage::new_test_storage;
    use actix_web::{test, App};

    fn request(platform: &str, model_name: &str) -> ModelNoteRequest {
        ModelNoteRequest {
            platform: platform.to_string(),
            model_name: model_name.to_string(),
            model_path: None,
            is_favorite: None,
            is_default: None,
            tags: None,
            notes: None,
        }
    }

    #[actix_web::test]
    async fn test_rejects_unknown_platform() {
        let (_dir, storage) = new_test_storage().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(create_or_update_model_note),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/model-notes")
            .set_json(request("openai", "gpt"))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 400);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(
            body["error"],
            "Invalid platform: openai. Must be 'llama' or 'ollama'"
        );
    }

    #[actix_web::test]
    async fn test_omitted_optional_fields_default_to_false_and_empty() {
        let (_dir, storage) = new_test_storage().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(create_or_update_model_note),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/model-notes")
            .set_json(request("llama", "minimal"))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: ModelNoteResponse = test::read_body_json(resp).await;
        assert_eq!(body.note.model_name, "minimal");
        assert!(!body.note.is_favorite);
        assert!(!body.note.is_default);
        assert!(body.note.tags.is_empty());
        assert!(body.note.notes.is_none());
        assert!(body.note.id.is_some());
    }

    #[actix_web::test]
    async fn test_persists_all_supplied_fields() {
        let (_dir, storage) = new_test_storage().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(create_or_update_model_note),
        )
        .await;

        let mut body_req = request("ollama", "llama3");
        body_req.model_path = Some("/models/llama3.gguf".to_string());
        body_req.is_favorite = Some(true);
        body_req.tags = Some(vec!["local".to_string(), "chat".to_string()]);
        body_req.notes = Some("good at summaries".to_string());

        let req = test::TestRequest::post()
            .uri("/api/model-notes")
            .set_json(&body_req)
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: ModelNoteResponse = test::read_body_json(resp).await;
        assert!(body.note.is_favorite);
        assert_eq!(
            body.note.model_path,
            Some("/models/llama3.gguf".to_string())
        );
        assert_eq!(body.note.tags, vec!["local", "chat"]);
        assert_eq!(body.note.notes.as_deref(), Some("good at summaries"));
    }

    #[actix_web::test]
    async fn test_default_request_drops_the_supplied_model_path() {
        let (_dir, storage) = new_test_storage().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(create_or_update_model_note),
        )
        .await;

        let mut body_req = request("llama", "the-default");
        body_req.is_default = Some(true);
        body_req.model_path = Some("/models/should-be-dropped.gguf".to_string());

        let req = test::TestRequest::post()
            .uri("/api/model-notes")
            .set_json(&body_req)
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: ModelNoteResponse = test::read_body_json(resp).await;
        assert!(body.note.is_default);
        assert!(body.note.model_path.is_none());
    }

    #[actix_web::test]
    async fn test_posting_the_same_model_twice_updates_in_place() {
        let (_dir, storage) = new_test_storage().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(create_or_update_model_note),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/model-notes")
            .set_json(request("llama", "twice"))
            .to_request();
        let first: ModelNoteResponse =
            test::read_body_json(test::call_service(&app, req).await).await;

        let mut second_req = request("llama", "twice");
        second_req.is_favorite = Some(true);
        let req = test::TestRequest::post()
            .uri("/api/model-notes")
            .set_json(&second_req)
            .to_request();
        let second: ModelNoteResponse =
            test::read_body_json(test::call_service(&app, req).await).await;

        assert_eq!(first.note.id, second.note.id);
        assert!(second.note.is_favorite);
    }

    #[actix_web::test]
    async fn test_malformed_json_body_is_rejected() {
        let (_dir, storage) = new_test_storage().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(create_or_update_model_note),
        )
        .await;

        // Missing the required `model_name` field.
        let req = test::TestRequest::post()
            .uri("/api/model-notes")
            .set_json(serde_json::json!({ "platform": "llama" }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_client_error());
    }

    #[actix_web::test]
    async fn test_storage_failure_returns_500() {
        let (_dir, storage) = new_test_storage().await;
        storage.drop_table_for_tests().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(create_or_update_model_note),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/model-notes")
            .set_json(request("llama", "doomed"))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 500);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body["error"]
            .as_str()
            .unwrap()
            .starts_with("Failed to save model note:"));
    }
}
