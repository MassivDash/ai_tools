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
