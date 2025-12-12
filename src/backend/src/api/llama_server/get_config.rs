use actix_web::{get, web, HttpResponse, Result as ActixResult};
use serde::Serialize;
use std::sync::{Arc, Mutex};

use crate::api::llama_server::types::Config;

#[derive(Serialize, Debug)]
pub struct ConfigResponse {
    pub hf_model: String,
    pub ctx_size: u32,
}

#[get("/api/llama-server/config")]
pub async fn get_llama_config(
    config: web::Data<Arc<Mutex<Config>>>,
) -> ActixResult<HttpResponse> {
    let config_guard = config.lock().unwrap();
    Ok(HttpResponse::Ok().json(ConfigResponse {
        hf_model: config_guard.hf_model.clone(),
        ctx_size: config_guard.ctx_size,
    }))
}

