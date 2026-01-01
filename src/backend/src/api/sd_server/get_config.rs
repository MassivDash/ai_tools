use crate::api::sd_server::types::SDConfigHandle;
use actix_web::{get, web, HttpResponse, Result as ActixResult};

#[get("/api/sd-server/config")]
pub async fn get_sd_config(config: web::Data<SDConfigHandle>) -> ActixResult<HttpResponse> {
    let config_guard = config.lock().unwrap();
    // Return the config directly since it now implements Serialize
    Ok(HttpResponse::Ok().json(&*config_guard))
}
