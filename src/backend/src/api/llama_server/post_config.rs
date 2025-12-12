use actix_web::{post, web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::Mutex;

use crate::api::llama_server::types::Config;

#[derive(Deserialize, Debug)]
pub struct ConfigRequest {
    pub hf_model: Option<String>,
    pub ctx_size: Option<u32>,
}

#[derive(Serialize, Debug)]
pub struct ConfigResponse {
    pub success: bool,
    pub message: String,
}

#[post("/api/llama-server/config")]
pub async fn post_update_config(
    body: web::Json<ConfigRequest>,
    config: web::Data<Arc<Mutex<Config>>>,
) -> ActixResult<HttpResponse> {
    let mut config_guard = config.lock().unwrap();

    if let Some(hf_model) = &body.hf_model {
        if !hf_model.trim().is_empty() {
            config_guard.hf_model = hf_model.trim().to_string();
            println!("📝 Updated HF model to: {}", config_guard.hf_model);
        }
    }

    if let Some(ctx_size) = body.ctx_size {
        if ctx_size > 0 {
            config_guard.ctx_size = ctx_size;
            println!("📝 Updated context size to: {}", config_guard.ctx_size);
        }
    }

    Ok(HttpResponse::Ok().json(ConfigResponse {
        success: true,
        message: "Configuration updated successfully".to_string(),
    }))
}

