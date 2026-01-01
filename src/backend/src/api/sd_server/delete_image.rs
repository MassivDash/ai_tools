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
