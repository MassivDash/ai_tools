use crate::api::sd_server::types::SDConfigHandle;
use actix_web::{get, web, HttpResponse, Result as ActixResult};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize, Debug)]
pub struct ImageInfo {
    pub filename: String,
    pub created: u64,
    pub path: String, // Relative path for serving
}

#[derive(Serialize, Debug)]
pub struct ImagesResponse {
    pub images: Vec<ImageInfo>,
}

#[get("/api/sd-server/images")]
pub async fn get_sd_images(config: web::Data<SDConfigHandle>) -> ActixResult<HttpResponse> {
    let config = config.lock().unwrap();
    // Assuming output_path is relative to models_path, or absolute.
    // The default is "./images", which is relative to models_path when running.
    // We need to list files in that folder.

    // We trust config.output_path to be where images are.
    // Default is "./public".

    let search_dir = Path::new(&config.output_path);
    let mut images = Vec::new();

    if search_dir.exists() {
        if let Ok(entries) = fs::read_dir(search_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        let ext = ext.to_string_lossy().to_lowercase();
                        if ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "webp" {
                            if let Ok(metadata) = entry.metadata() {
                                let created = metadata
                                    .created()
                                    .ok()
                                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                    .map(|d| d.as_secs())
                                    .unwrap_or(0);

                                if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                                    images.push(ImageInfo {
                                        filename: filename.to_string(),
                                        created,
                                        // Assume we are serving output_path (./public) at /public
                                        path: format!("/public/{}", filename),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort by newest first
    images.sort_by(|a, b| b.created.cmp(&a.created));

    Ok(HttpResponse::Ok().json(ImagesResponse { images }))
}
