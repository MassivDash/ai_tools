use actix_web::{post, web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};

use crate::api::sd_server::types::SDConfigHandle;

#[derive(Deserialize, Serialize, Debug)]
pub struct SDConfigRequest {
    // CLI Options
    pub output_path: Option<String>,
    pub verbose: Option<bool>,
    pub mode: Option<String>,

    // Context Options
    pub diffusion_model: Option<String>,
    pub model: Option<String>,
    pub clip_l: Option<String>,
    pub clip_g: Option<String>,
    pub t5xxl: Option<String>,
    pub llm: Option<String>,
    pub vae: Option<String>,
    pub control_net: Option<String>,
    pub lora_model_dir: Option<String>,
    pub threads: Option<i32>,
    pub offload_to_cpu: Option<bool>,
    pub diffusion_fa: Option<bool>,
    pub control_net_cpu: Option<bool>,
    pub clip_on_cpu: Option<bool>,
    pub vae_on_cpu: Option<bool>,
    pub vae_tiling: Option<bool>,
    pub vae_tile_size: Option<u32>,
    pub vae_relative_tile_size: Option<f32>,
    pub models_path: Option<String>,
    pub rng: Option<String>,

    // Generation Options
    pub prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub init_img: Option<String>,
    pub height: Option<u32>,
    pub width: Option<u32>,
    pub steps: Option<u32>,
    pub batch_count: Option<u32>,
    pub cfg_scale: Option<f32>,
    pub guidance: Option<f32>,
    pub strength: Option<f32>,
    pub seed: Option<i64>,
    pub sampling_method: Option<String>,
    pub scheduler: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SDConfigResponse {
    pub success: bool,
    pub message: String,
}

#[post("/api/sd-server/config")]
pub async fn post_update_sd_config(
    body: web::Json<SDConfigRequest>,
    config: web::Data<SDConfigHandle>,
) -> ActixResult<HttpResponse> {
    let mut config_guard = config.lock().unwrap();

    // Mapping fields
    if let Some(v) = &body.output_path {
        config_guard.output_path = v.clone();
    }
    if let Some(v) = body.verbose {
        config_guard.verbose = v;
    }
    if let Some(v) = &body.mode {
        config_guard.mode = Some(v.clone());
    }

    if let Some(v) = &body.diffusion_model {
        config_guard.diffusion_model = v.clone();
    }
    if let Some(v) = &body.model {
        config_guard.model = Some(v.clone());
    }
    if let Some(v) = &body.clip_l {
        config_guard.clip_l = Some(v.clone());
    }
    if let Some(v) = &body.clip_g {
        config_guard.clip_g = Some(v.clone());
    }
    if let Some(v) = &body.t5xxl {
        config_guard.t5xxl = Some(v.clone());
    }
    if let Some(v) = &body.llm {
        config_guard.llm = Some(v.clone());
    }
    if let Some(v) = &body.vae {
        config_guard.vae = Some(v.clone());
    }
    if let Some(v) = &body.control_net {
        config_guard.control_net = Some(v.clone());
    }
    if let Some(v) = &body.lora_model_dir {
        config_guard.lora_model_dir = Some(v.clone());
    }
    if let Some(v) = body.threads {
        config_guard.threads = v;
    }

    if let Some(v) = body.offload_to_cpu {
        config_guard.offload_to_cpu = v;
    }
    if let Some(v) = body.diffusion_fa {
        config_guard.diffusion_fa = v;
    }
    if let Some(v) = body.control_net_cpu {
        config_guard.control_net_cpu = v;
    }
    if let Some(v) = body.clip_on_cpu {
        config_guard.clip_on_cpu = v;
    }
    if let Some(v) = body.vae_on_cpu {
        config_guard.vae_on_cpu = v;
    }
    if let Some(v) = body.vae_tiling {
        config_guard.vae_tiling = v;
    }
    if let Some(v) = body.vae_tile_size {
        config_guard.vae_tile_size = Some(v);
    }
    if let Some(v) = body.vae_relative_tile_size {
        config_guard.vae_relative_tile_size = Some(v);
    }
    if let Some(v) = &body.models_path {
        config_guard.models_path = v.clone();
    }
    if let Some(v) = &body.rng {
        config_guard.rng = v.clone();
    }

    if let Some(v) = &body.prompt {
        config_guard.prompt = v.clone();
    }
    if let Some(v) = &body.negative_prompt {
        config_guard.negative_prompt = v.clone();
    }
    if let Some(v) = &body.init_img {
        config_guard.init_img = Some(v.clone());
    }
    if let Some(v) = body.height {
        config_guard.height = v;
    }
    if let Some(v) = body.width {
        config_guard.width = v;
    }
    if let Some(v) = body.steps {
        config_guard.steps = Some(v);
    }
    if let Some(v) = body.batch_count {
        config_guard.batch_count = Some(v);
    }
    if let Some(v) = body.cfg_scale {
        config_guard.cfg_scale = v;
    }
    if let Some(v) = body.guidance {
        config_guard.guidance = Some(v);
    }
    if let Some(v) = body.strength {
        config_guard.strength = Some(v);
    }
    if let Some(v) = body.seed {
        config_guard.seed = Some(v);
    }
    if let Some(v) = &body.sampling_method {
        config_guard.sampling_method = Some(v.clone());
    }
    if let Some(v) = &body.scheduler {
        config_guard.scheduler = Some(v.clone());
    }

    Ok(HttpResponse::Ok().json(SDConfigResponse {
        success: true,
        message: "SD Configuration updated successfully".to_string(),
    }))
}
