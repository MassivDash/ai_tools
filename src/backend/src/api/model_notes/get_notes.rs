use crate::api::model_notes::sqlite_storage::ModelNotesStorage;
use crate::api::model_notes::types::ModelNotesResponse;
use actix_web::{get, web, HttpResponse, Result as ActixResult};
use std::sync::Arc;

#[get("/api/model-notes")]
pub async fn get_model_notes(
    storage: web::Data<Arc<ModelNotesStorage>>,
) -> ActixResult<HttpResponse> {
    println!("📋 Fetching all model notes...");

    match storage.get_all_notes().await {
        Ok(notes) => {
            println!("✅ Found {} model notes", notes.len());
            Ok(HttpResponse::Ok().json(ModelNotesResponse { notes }))
        }
        Err(e) => {
            println!("Failed to fetch model notes: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to fetch model notes: {}", e)
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

    fn note(platform: &str, model_name: &str, favorite: bool) -> ModelNote {
        ModelNote {
            id: None,
            platform: platform.to_string(),
            model_name: model_name.to_string(),
            model_path: None,
            is_favorite: favorite,
            is_default: false,
            tags: vec!["tag".to_string()],
            notes: Some("a note".to_string()),
            created_at: None,
            updated_at: None,
        }
    }

    #[actix_web::test]
    async fn test_get_model_notes_empty() {
        let (_dir, storage) = new_test_storage().await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(get_model_notes),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/model-notes")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: ModelNotesResponse = test::read_body_json(resp).await;
        assert!(body.notes.is_empty());
    }

    #[actix_web::test]
    async fn test_get_model_notes_returns_stored_notes_favorites_first() {
        let (_dir, storage) = new_test_storage().await;
        storage
            .upsert_note(&note("llama", "plain", false))
            .await
            .unwrap();
        storage
            .upsert_note(&note("ollama", "starred", true))
            .await
            .unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(get_model_notes),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/model-notes")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: ModelNotesResponse = test::read_body_json(resp).await;
        assert_eq!(body.notes.len(), 2);
        assert_eq!(body.notes[0].model_name, "starred");
        assert_eq!(body.notes[0].tags, vec!["tag"]);
        assert_eq!(body.notes[1].model_name, "plain");
    }

    #[actix_web::test]
    async fn test_get_model_notes_storage_failure_returns_500() {
        let (_dir, storage) = new_test_storage().await;
        storage.drop_table_for_tests().await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(get_model_notes),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/model-notes")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 500);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body["error"]
            .as_str()
            .unwrap()
            .starts_with("Failed to fetch model notes:"));
    }
}
