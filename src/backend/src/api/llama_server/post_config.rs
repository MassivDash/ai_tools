use actix_web::{post, web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::Mutex;

use crate::api::llama_server::types::Config;

#[derive(Deserialize, Debug)]
pub struct ConfigRequest {
    pub hf_model: Option<String>,
    pub ctx_size: Option<u32>,
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

    if let Some(threads) = body.threads {
        config_guard.threads = Some(threads);
        println!("📝 Updated threads to: {:?}", config_guard.threads);
    }

    if let Some(threads_batch) = body.threads_batch {
        config_guard.threads_batch = Some(threads_batch);
        println!("📝 Updated threads-batch to: {:?}", config_guard.threads_batch);
    }

    if let Some(predict) = body.predict {
        config_guard.predict = Some(predict);
        println!("📝 Updated predict to: {:?}", config_guard.predict);
    }

    if let Some(batch_size) = body.batch_size {
        if batch_size > 0 {
            config_guard.batch_size = Some(batch_size);
            println!("📝 Updated batch-size to: {:?}", config_guard.batch_size);
        }
    }

    if let Some(ubatch_size) = body.ubatch_size {
        if ubatch_size > 0 {
            config_guard.ubatch_size = Some(ubatch_size);
            println!("📝 Updated ubatch-size to: {:?}", config_guard.ubatch_size);
        }
    }

    if let Some(flash_attn) = body.flash_attn {
        config_guard.flash_attn = Some(flash_attn);
        println!("📝 Updated flash-attn to: {:?}", config_guard.flash_attn);
    }

    if let Some(mlock) = body.mlock {
        config_guard.mlock = Some(mlock);
        println!("📝 Updated mlock to: {:?}", config_guard.mlock);
    }

    if let Some(no_mmap) = body.no_mmap {
        config_guard.no_mmap = Some(no_mmap);
        println!("📝 Updated no-mmap to: {:?}", config_guard.no_mmap);
    }

    if let Some(gpu_layers) = body.gpu_layers {
        config_guard.gpu_layers = Some(gpu_layers);
        println!("📝 Updated gpu-layers to: {:?}", config_guard.gpu_layers);
    }

    if let Some(model) = &body.model {
        if !model.trim().is_empty() {
            config_guard.model = Some(model.trim().to_string());
            println!("📝 Updated model to: {:?}", config_guard.model);
        }
    }

    Ok(HttpResponse::Ok().json(ConfigResponse {
        success: true,
        message: "Configuration updated successfully".to_string(),
    }))
}

