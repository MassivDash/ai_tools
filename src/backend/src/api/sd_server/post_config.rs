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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::sd_server::types::SDConfig;
    use actix_web::{test, App};
    use std::sync::{Arc, Mutex};

    async fn post(config: &SDConfigHandle, body: serde_json::Value) -> u16 {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config.clone()))
                .service(post_update_sd_config),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/sd-server/config")
            .set_json(&body)
            .to_request();
        test::call_service(&app, req).await.status().as_u16()
    }

    #[actix_web::test]
    async fn test_empty_body_leaves_the_config_untouched() {
        let config: SDConfigHandle = Arc::new(Mutex::new(SDConfig::default()));
        let before = config.lock().unwrap().clone();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config.clone()))
                .service(post_update_sd_config),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/sd-server/config")
            .set_json(serde_json::json!({}))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: SDConfigResponse = test::read_body_json(resp).await;
        assert!(body.success);
        assert_eq!(body.message, "SD Configuration updated successfully");

        let after = config.lock().unwrap();
        assert_eq!(after.prompt, before.prompt);
        assert_eq!(after.width, before.width);
        assert_eq!(after.diffusion_model, before.diffusion_model);
        assert_eq!(after.steps, before.steps);
        assert_eq!(after.verbose, before.verbose);
    }

    #[actix_web::test]
    async fn test_updates_every_string_field() {
        let config: SDConfigHandle = Arc::new(Mutex::new(SDConfig::default()));

        let status = post(
            &config,
            serde_json::json!({
                "output_path": "/out",
                "mode": "img_gen",
                "diffusion_model": "flux.gguf",
                "model": "/models/full.safetensors",
                "clip_l": "clip_l.gguf",
                "clip_g": "clip_g.gguf",
                "t5xxl": "t5.gguf",
                "llm": "qwen.gguf",
                "vae": "ae.safetensors",
                "control_net": "cn.safetensors",
                "lora_model_dir": "/loras",
                "models_path": "/models",
                "rng": "cuda",
                "prompt": "a fox",
                "negative_prompt": "blurry",
                "init_img": "/in.png",
                "sampling_method": "euler_a",
                "scheduler": "karras",
            }),
        )
        .await;
        assert_eq!(status, 200);

        let c = config.lock().unwrap();
        assert_eq!(c.output_path, "/out");
        assert_eq!(c.mode.as_deref(), Some("img_gen"));
        assert_eq!(c.diffusion_model, "flux.gguf");
        assert_eq!(c.model.as_deref(), Some("/models/full.safetensors"));
        assert_eq!(c.clip_l.as_deref(), Some("clip_l.gguf"));
        assert_eq!(c.clip_g.as_deref(), Some("clip_g.gguf"));
        assert_eq!(c.t5xxl.as_deref(), Some("t5.gguf"));
        assert_eq!(c.llm.as_deref(), Some("qwen.gguf"));
        assert_eq!(c.vae.as_deref(), Some("ae.safetensors"));
        assert_eq!(c.control_net.as_deref(), Some("cn.safetensors"));
        assert_eq!(c.lora_model_dir.as_deref(), Some("/loras"));
        assert_eq!(c.models_path, "/models");
        assert_eq!(c.rng, "cuda");
        assert_eq!(c.prompt, "a fox");
        assert_eq!(c.negative_prompt, "blurry");
        assert_eq!(c.init_img.as_deref(), Some("/in.png"));
        assert_eq!(c.sampling_method.as_deref(), Some("euler_a"));
        assert_eq!(c.scheduler.as_deref(), Some("karras"));
    }

    #[actix_web::test]
    async fn test_updates_every_boolean_and_numeric_field() {
        let config: SDConfigHandle = Arc::new(Mutex::new(SDConfig::default()));

        let status = post(
            &config,
            serde_json::json!({
                "verbose": false,
                "threads": 8,
                "offload_to_cpu": false,
                "diffusion_fa": false,
                "control_net_cpu": false,
                "clip_on_cpu": false,
                "vae_on_cpu": false,
                "vae_tiling": false,
                "vae_tile_size": 512,
                "vae_relative_tile_size": 0.5,
                "height": 768,
                "width": 512,
                "steps": 25,
                "batch_count": 4,
                "cfg_scale": 7.5,
                "guidance": 3.5,
                "strength": 0.75,
                "seed": 12345,
            }),
        )
        .await;
        assert_eq!(status, 200);

        let c = config.lock().unwrap();
        assert!(!c.verbose);
        assert_eq!(c.threads, 8);
        assert!(!c.offload_to_cpu);
        assert!(!c.diffusion_fa);
        assert!(!c.control_net_cpu);
        assert!(!c.clip_on_cpu);
        assert!(!c.vae_on_cpu);
        assert!(!c.vae_tiling);
        assert_eq!(c.vae_tile_size, Some(512));
        assert_eq!(c.vae_relative_tile_size, Some(0.5));
        assert_eq!(c.height, 768);
        assert_eq!(c.width, 512);
        assert_eq!(c.steps, Some(25));
        assert_eq!(c.batch_count, Some(4));
        assert_eq!(c.cfg_scale, 7.5);
        assert_eq!(c.guidance, Some(3.5));
        assert_eq!(c.strength, Some(0.75));
        assert_eq!(c.seed, Some(12345));
    }

    #[actix_web::test]
    async fn test_explicit_nulls_do_not_clear_existing_values() {
        let config: SDConfigHandle = Arc::new(Mutex::new(SDConfig::default()));
        {
            let mut guard = config.lock().unwrap();
            guard.steps = Some(40);
            guard.vae = Some("keep.safetensors".to_string());
        }

        // Every request field is an Option, and the handler only assigns on
        // `Some(..)`, so a JSON null is indistinguishable from an omitted field.
        let status = post(&config, serde_json::json!({ "steps": null, "vae": null })).await;
        assert_eq!(status, 200);

        let c = config.lock().unwrap();
        assert_eq!(c.steps, Some(40));
        assert_eq!(c.vae.as_deref(), Some("keep.safetensors"));
    }

    #[actix_web::test]
    async fn test_partial_update_only_touches_the_supplied_fields() {
        let config: SDConfigHandle = Arc::new(Mutex::new(SDConfig::default()));

        let status = post(&config, serde_json::json!({ "prompt": "just the prompt" })).await;
        assert_eq!(status, 200);

        let c = config.lock().unwrap();
        assert_eq!(c.prompt, "just the prompt");
        assert_eq!(c.negative_prompt, "");
        assert_eq!(c.width, 1024);
        assert_eq!(c.models_path, "./sd_models");
    }

    #[actix_web::test]
    async fn test_wrong_field_type_is_rejected() {
        let config: SDConfigHandle = Arc::new(Mutex::new(SDConfig::default()));

        let status = post(&config, serde_json::json!({ "width": "not a number" })).await;
        assert_eq!(status, 400);

        // The config must be left untouched by a rejected request.
        assert_eq!(config.lock().unwrap().width, 1024);
    }
}
