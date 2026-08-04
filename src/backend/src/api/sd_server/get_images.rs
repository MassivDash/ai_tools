use actix_web::{get, web, HttpResponse, Result as ActixResult};
use serde::Serialize;
use std::sync::Arc;

use crate::api::sd_server::storage::SDImagesStorage;

#[derive(Serialize, Debug)]
pub struct ImageInfo {
    pub filename: String,
    pub created: u64,
    pub path: String, // Relative path for serving
    pub prompt: String,
    pub diffusion_model: String,
    pub width: i64,
    pub height: i64,
    pub steps: Option<i64>,
    pub cfg_scale: f32,
    pub seed: Option<i64>,
    pub additional_info: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct ImagesResponse {
    pub images: Vec<ImageInfo>,
}

#[get("/api/sd-server/images")]
pub async fn get_sd_images(storage: web::Data<Arc<SDImagesStorage>>) -> ActixResult<HttpResponse> {
    let images_metadata = match storage.get_images().await {
        Ok(imgs) => imgs,
        Err(e) => {
            eprintln!("Failed to fetch images from DB: {:?}", e);
            return Ok(HttpResponse::InternalServerError().body("DB Error"));
        }
    };

    let images = images_metadata
        .into_iter()
        .map(|meta| {
            ImageInfo {
                filename: meta.filename.clone(),
                created: meta.created_at as u64, // DB stores as i64
                path: format!("/public/{}", meta.filename),
                prompt: meta.prompt,
                diffusion_model: meta.diffusion_model,
                width: meta.width,
                height: meta.height,
                steps: meta.steps,
                cfg_scale: meta.cfg_scale,
                seed: meta.seed,
                additional_info: meta.additional_info,
            }
        })
        .collect();

    Ok(HttpResponse::Ok().json(ImagesResponse { images }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::sd_server::storage::{new_broken_test_storage, new_test_storage, test_image};
    use actix_web::{test, App};

    #[derive(serde::Deserialize)]
    struct ImagesResponseBody {
        images: Vec<ImageInfoBody>,
    }

    #[derive(serde::Deserialize)]
    struct ImageInfoBody {
        filename: String,
        created: u64,
        path: String,
        prompt: String,
        diffusion_model: String,
        width: i64,
        height: i64,
        steps: Option<i64>,
        cfg_scale: f32,
        seed: Option<i64>,
        additional_info: Option<String>,
    }

    #[actix_web::test]
    async fn test_get_sd_images_empty() {
        let storage = new_test_storage().await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(get_sd_images),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/sd-server/images")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: ImagesResponseBody = test::read_body_json(resp).await;
        assert!(body.images.is_empty());
    }

    #[actix_web::test]
    async fn test_get_sd_images_maps_metadata_and_builds_public_path() {
        let storage = new_test_storage().await;
        storage
            .add_image(test_image("cat.png", 1234))
            .await
            .unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(get_sd_images),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/sd-server/images")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: ImagesResponseBody = test::read_body_json(resp).await;
        assert_eq!(body.images.len(), 1);
        let img = &body.images[0];
        assert_eq!(img.filename, "cat.png");
        assert_eq!(img.path, "/public/cat.png");
        assert_eq!(img.created, 1234);
        assert_eq!(img.prompt, "a cat");
        assert_eq!(img.diffusion_model, "model.gguf");
        assert_eq!(img.width, 512);
        assert_eq!(img.height, 768);
        assert_eq!(img.steps, Some(20));
        assert_eq!(img.cfg_scale, 1.5);
        assert_eq!(img.seed, Some(42));
        assert_eq!(
            img.additional_info.as_deref(),
            Some(r#"{"sampler":"euler"}"#)
        );
    }

    #[actix_web::test]
    async fn test_get_sd_images_preserves_newest_first_ordering() {
        let storage = new_test_storage().await;
        storage.add_image(test_image("old.png", 10)).await.unwrap();
        storage.add_image(test_image("new.png", 20)).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(get_sd_images),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/sd-server/images")
            .to_request();
        let resp = test::call_service(&app, req).await;

        let body: ImagesResponseBody = test::read_body_json(resp).await;
        let names: Vec<&str> = body.images.iter().map(|i| i.filename.as_str()).collect();
        assert_eq!(names, vec!["new.png", "old.png"]);
    }

    #[actix_web::test]
    async fn test_get_sd_images_storage_failure_returns_500() {
        let storage = new_broken_test_storage().await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(get_sd_images),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/sd-server/images")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 500);
        assert_eq!(test::read_body(resp).await, "DB Error");
    }
}
