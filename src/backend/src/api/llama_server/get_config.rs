use actix_web::{get, web, HttpResponse, Result as ActixResult};
use serde::Serialize;
use std::sync::{Arc, Mutex};

use crate::api::llama_server::types::Config;

#[derive(Serialize, Debug)]
pub struct ConfigResponse {
    pub hf_model: String,
    pub ctx_size: u32,
    pub threads: Option<i32>,
    pub threads_batch: Option<i32>,
    pub predict: Option<i32>,
    pub batch_size: Option<u32>,
    pub ubatch_size: Option<u32>,
    pub flash_attn: Option<bool>,
    pub mlock: Option<bool>,
    pub no_mmap: Option<bool>,
    pub gpu_layers: Option<u32>,
    pub model: Option<String>,
}

#[get("/api/llama-server/config")]
pub async fn get_llama_config(config: web::Data<Arc<Mutex<Config>>>) -> ActixResult<HttpResponse> {
    let config_guard = config.lock().unwrap();
    Ok(HttpResponse::Ok().json(ConfigResponse {
        hf_model: config_guard.hf_model.clone(),
        ctx_size: config_guard.ctx_size,
        threads: config_guard.threads,
        threads_batch: config_guard.threads_batch,
        predict: config_guard.predict,
        batch_size: config_guard.batch_size,
        ubatch_size: config_guard.ubatch_size,
        flash_attn: config_guard.flash_attn,
        mlock: config_guard.mlock,
        no_mmap: config_guard.no_mmap,
        gpu_layers: config_guard.gpu_layers,
        model: config_guard.model.clone(),
    }))
}
