use crate::api::sd_server::storage::SDImagesStorage;
use crate::api::sd_server::types::SDConfigHandle;
use actix_web::{delete, web, HttpResponse, Result as ActixResult};
use std::path::Path;
use std::sync::Arc;

#[delete("/api/sd-server/image/{filename}")]
pub async fn delete_sd_image(
    filename: web::Path<String>,
    storage: web::Data<Arc<SDImagesStorage>>,
    config: web::Data<SDConfigHandle>,
) -> ActixResult<HttpResponse> {
    let filename_str = filename.into_inner();

    // 1. Delete from DB
    if let Err(e) = storage.delete_image(&filename_str).await {
        eprintln!("Failed to delete image from DB: {:?}", e);
        return Ok(HttpResponse::InternalServerError().body("DB delete failed"));
    }

    // 2. Delete from Filesystem
    let config = config.lock().unwrap();
    let image_path = Path::new(&config.output_path).join(&filename_str);

    // Check if exists before trying to delete (to avoid error if manually deleted)
    if image_path.exists() {
        if let Err(e) = std::fs::remove_file(&image_path) {
            eprintln!("Failed to delete image file: {:?}", e);
            // We successfully deleted from DB, so we can consider this a partial success or warning.
            // But let's return OK since the "resource" is effectively gone from the app's view.
        }
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({ "success": true })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::sd_server::storage::{new_broken_test_storage, new_test_storage, test_image};
    use crate::api::sd_server::types::SDConfig;
    use actix_web::{test, App};
    use std::sync::Mutex;

    fn config_with_output(output_path: &str) -> SDConfigHandle {
        Arc::new(Mutex::new(SDConfig {
            output_path: output_path.to_string(),
            ..SDConfig::default()
        }))
    }

    #[actix_web::test]
    async fn test_delete_removes_both_the_row_and_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let image_path = dir.path().join("cat.png");
        std::fs::write(&image_path, b"not really a png").unwrap();

        let storage = new_test_storage().await;
        storage.add_image(test_image("cat.png", 1)).await.unwrap();
        let storage = Arc::new(storage);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(storage.clone()))
                .app_data(web::Data::new(config_with_output(
                    dir.path().to_str().unwrap(),
                )))
                .service(delete_sd_image),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri("/api/sd-server/image/cat.png")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);

        assert!(storage.get_images().await.unwrap().is_empty());
        assert!(!image_path.exists());
    }

    #[actix_web::test]
    async fn test_delete_succeeds_when_the_file_is_already_gone() {
        let dir = tempfile::tempdir().unwrap();
        let storage = new_test_storage().await;
        storage.add_image(test_image("ghost.png", 1)).await.unwrap();
        let storage = Arc::new(storage);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(storage.clone()))
                .app_data(web::Data::new(config_with_output(
                    dir.path().to_str().unwrap(),
                )))
                .service(delete_sd_image),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri("/api/sd-server/image/ghost.png")
            .to_request();
        let resp = test::call_service(&app, req).await;

        // A missing file on disk is not an error - the DB row is what matters.
        assert_eq!(resp.status().as_u16(), 200);
        assert!(storage.get_images().await.unwrap().is_empty());
    }

    #[actix_web::test]
    async fn test_delete_leaves_other_images_alone() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("keep.png"), b"x").unwrap();
        std::fs::write(dir.path().join("drop.png"), b"x").unwrap();

        let storage = new_test_storage().await;
        storage.add_image(test_image("keep.png", 1)).await.unwrap();
        storage.add_image(test_image("drop.png", 2)).await.unwrap();
        let storage = Arc::new(storage);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(storage.clone()))
                .app_data(web::Data::new(config_with_output(
                    dir.path().to_str().unwrap(),
                )))
                .service(delete_sd_image),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri("/api/sd-server/image/drop.png")
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status().as_u16(), 200);

        let remaining = storage.get_images().await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].filename, "keep.png");
        assert!(dir.path().join("keep.png").exists());
        assert!(!dir.path().join("drop.png").exists());
    }

    #[actix_web::test]
    async fn test_delete_returns_500_and_keeps_the_file_when_the_db_fails() {
        let dir = tempfile::tempdir().unwrap();
        let image_path = dir.path().join("cat.png");
        std::fs::write(&image_path, b"x").unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(new_broken_test_storage().await)))
                .app_data(web::Data::new(config_with_output(
                    dir.path().to_str().unwrap(),
                )))
                .service(delete_sd_image),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri("/api/sd-server/image/cat.png")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 500);
        assert_eq!(test::read_body(resp).await, "DB delete failed");
        // The handler bails out before touching the filesystem.
        assert!(image_path.exists());
    }
}
