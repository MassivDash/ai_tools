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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::sd_server::model_sets::{
        new_broken_test_storage, new_test_storage, SDModelSet,
    };
    use actix_web::{test, App};

    macro_rules! app_for {
        ($storage:expr, $($service:expr),+) => {
            test::init_service(
                App::new()
                    .app_data(web::Data::new($storage))
                    $(.service($service))+
            )
            .await
        };
    }

    #[actix_web::test]
    async fn test_list_model_sets_empty() {
        let app = app_for!(Arc::new(new_test_storage().await), list_model_sets);

        let req = test::TestRequest::get()
            .uri("/api/sd-server/model-sets")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: Vec<SDModelSet> = test::read_body_json(resp).await;
        assert!(body.is_empty());
    }

    #[actix_web::test]
    async fn test_create_then_list_round_trip() {
        let app = app_for!(
            Arc::new(new_test_storage().await),
            create_model_set,
            list_model_sets
        );

        let req = test::TestRequest::post()
            .uri("/api/sd-server/model-sets")
            .set_json(serde_json::json!({
                "name": "flux",
                "diffusion_model": "flux1-dev.gguf",
                "vae": "ae.safetensors",
                "llm": null,
                "is_default": true,
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let created: SDModelSet = test::read_body_json(resp).await;
        assert_eq!(created.name, "flux");
        assert_eq!(created.vae.as_deref(), Some("ae.safetensors"));
        assert!(created.llm.is_none());
        assert!(created.is_default);

        let req = test::TestRequest::get()
            .uri("/api/sd-server/model-sets")
            .to_request();
        let listed: Vec<SDModelSet> =
            test::read_body_json(test::call_service(&app, req).await).await;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, created.id);
    }

    #[actix_web::test]
    async fn test_create_requires_the_mandatory_fields() {
        let app = app_for!(Arc::new(new_test_storage().await), create_model_set);

        // `diffusion_model` and `is_default` are missing.
        let req = test::TestRequest::post()
            .uri("/api/sd-server/model-sets")
            .set_json(serde_json::json!({ "name": "incomplete" }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_client_error());
    }

    #[actix_web::test]
    async fn test_update_model_set_persists_the_change() {
        let storage = Arc::new(new_test_storage().await);
        let created = storage
            .create(
                "before".to_string(),
                "a.gguf".to_string(),
                None,
                None,
                false,
            )
            .await
            .unwrap();

        let app = app_for!(storage.clone(), update_model_set);

        let req = test::TestRequest::put()
            .uri(&format!("/api/sd-server/model-sets/{}", created.id))
            .set_json(serde_json::json!({
                "name": "after",
                "diffusion_model": "b.gguf",
                "vae": null,
                "llm": "t5.gguf",
                "is_default": true,
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: String = test::read_body_json(resp).await;
        assert_eq!(body, "Updated");

        let stored = &storage.list().await.unwrap()[0];
        assert_eq!(stored.name, "after");
        assert_eq!(stored.diffusion_model, "b.gguf");
        assert_eq!(stored.llm.as_deref(), Some("t5.gguf"));
        assert!(stored.is_default);
    }

    #[actix_web::test]
    async fn test_update_with_a_non_numeric_id_is_rejected() {
        let app = app_for!(Arc::new(new_test_storage().await), update_model_set);

        let req = test::TestRequest::put()
            .uri("/api/sd-server/model-sets/not-a-number")
            .set_json(serde_json::json!({
                "name": "x",
                "diffusion_model": "x.gguf",
                "is_default": false,
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_client_error());
    }

    #[actix_web::test]
    async fn test_delete_model_set_removes_the_row() {
        let storage = Arc::new(new_test_storage().await);
        let created = storage
            .create(
                "doomed".to_string(),
                "a.gguf".to_string(),
                None,
                None,
                false,
            )
            .await
            .unwrap();

        let app = app_for!(storage.clone(), delete_model_set);

        let req = test::TestRequest::delete()
            .uri(&format!("/api/sd-server/model-sets/{}", created.id))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: String = test::read_body_json(resp).await;
        assert_eq!(body, "Deleted");
        assert!(storage.list().await.unwrap().is_empty());
    }

    #[actix_web::test]
    async fn test_delete_of_an_unknown_id_still_reports_success() {
        let app = app_for!(Arc::new(new_test_storage().await), delete_model_set);

        let req = test::TestRequest::delete()
            .uri("/api/sd-server/model-sets/9999")
            .to_request();
        let resp = test::call_service(&app, req).await;

        // The DELETE statement simply matches no rows.
        assert_eq!(resp.status().as_u16(), 200);
    }

    #[actix_web::test]
    async fn test_all_handlers_return_500_when_the_table_is_missing() {
        let app = app_for!(
            Arc::new(new_broken_test_storage().await),
            list_model_sets,
            create_model_set,
            update_model_set,
            delete_model_set
        );

        let req = test::TestRequest::get()
            .uri("/api/sd-server/model-sets")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 500);
        let body: String = test::read_body_json(resp).await;
        assert!(body.starts_with("Failed to list sets:"));

        let req = test::TestRequest::post()
            .uri("/api/sd-server/model-sets")
            .set_json(serde_json::json!({
                "name": "x",
                "diffusion_model": "x.gguf",
                "is_default": false,
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 500);
        let body: String = test::read_body_json(resp).await;
        assert!(body.starts_with("Failed to create set:"));

        let req = test::TestRequest::put()
            .uri("/api/sd-server/model-sets/1")
            .set_json(serde_json::json!({
                "name": "x",
                "diffusion_model": "x.gguf",
                "is_default": false,
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 500);
        let body: String = test::read_body_json(resp).await;
        assert!(body.starts_with("Failed to update set:"));

        let req = test::TestRequest::delete()
            .uri("/api/sd-server/model-sets/1")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 500);
        let body: String = test::read_body_json(resp).await;
        assert!(body.starts_with("Failed to delete set:"));
    }
}
