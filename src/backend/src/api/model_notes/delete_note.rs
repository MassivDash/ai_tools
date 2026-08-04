use crate::api::model_notes::sqlite_storage::ModelNotesStorage;
use actix_web::{delete, web, HttpResponse, Result as ActixResult};
use std::sync::Arc;

#[delete("/api/model-notes/{platform}/{model_name}")]
pub async fn delete_model_note(
    path: web::Path<(String, String)>,
    storage: web::Data<Arc<ModelNotesStorage>>,
) -> ActixResult<HttpResponse> {
    let (platform, model_name) = path.into_inner();
    println!("🗑️  Deleting model note for {}:{}", platform, model_name);

    match storage.delete_note(&platform, &model_name).await {
        Ok(true) => {
            println!("✅ Successfully deleted model note");
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "Model note deleted successfully"
            })))
        }
        Ok(false) => {
            println!("⚠️  Model note not found");
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "Model note not found"
            })))
        }
        Err(e) => {
            println!("Failed to delete model note: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to delete model note: {}", e)
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

    fn note(platform: &str, model_name: &str) -> ModelNote {
        ModelNote {
            id: None,
            platform: platform.to_string(),
            model_name: model_name.to_string(),
            model_path: None,
            is_favorite: false,
            is_default: false,
            tags: Vec::new(),
            notes: None,
            created_at: None,
            updated_at: None,
        }
    }

    #[actix_web::test]
    async fn test_delete_existing_note_removes_the_row() {
        let (_dir, storage) = new_test_storage().await;
        storage.upsert_note(&note("llama", "doomed")).await.unwrap();
        let storage = Arc::new(storage);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(storage.clone()))
                .service(delete_model_note),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri("/api/model-notes/llama/doomed")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["message"], "Model note deleted successfully");

        // The row really is gone.
        assert!(storage.get_note("llama", "doomed").await.unwrap().is_none());
    }

    #[actix_web::test]
    async fn test_delete_missing_note_returns_404() {
        let (_dir, storage) = new_test_storage().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(delete_model_note),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri("/api/model-notes/llama/never-existed")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 404);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Model note not found");
    }

    #[actix_web::test]
    async fn test_delete_is_scoped_to_the_platform_in_the_path() {
        let (_dir, storage) = new_test_storage().await;
        storage.upsert_note(&note("llama", "shared")).await.unwrap();
        storage
            .upsert_note(&note("ollama", "shared"))
            .await
            .unwrap();
        let storage = Arc::new(storage);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(storage.clone()))
                .service(delete_model_note),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri("/api/model-notes/llama/shared")
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status().as_u16(), 200);

        assert!(storage.get_note("llama", "shared").await.unwrap().is_none());
        assert!(storage
            .get_note("ollama", "shared")
            .await
            .unwrap()
            .is_some());
    }

    #[actix_web::test]
    async fn test_delete_storage_failure_returns_500() {
        let (_dir, storage) = new_test_storage().await;
        storage.drop_table_for_tests().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(delete_model_note),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri("/api/model-notes/llama/anything")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 500);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body["error"]
            .as_str()
            .unwrap()
            .starts_with("Failed to delete model note:"));
    }
}
