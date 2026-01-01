use actix_web::{post, web, HttpResponse, Result as ActixResult};
use serde::Serialize;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::api::sd_server::types::{SDConfigHandle, SDProcessHandle};

#[derive(Serialize, Debug)]
pub struct SDServerResponse {
    pub success: bool,
    pub message: String,
}

#[post("/api/sd-server/start")]
pub async fn post_start_sd_server(
    process: web::Data<SDProcessHandle>,
    config: web::Data<SDConfigHandle>,
) -> ActixResult<HttpResponse> {
    let mut process_guard = process.lock().unwrap();

    if let Some(ref mut child) = *process_guard {
        match child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => {
                return Ok(HttpResponse::Ok().json(SDServerResponse {
                    success: false,
                    message: "SD generation is already running".to_string(),
                }));
            }
            Err(_) => {}
        }
    }

    let config = config.lock().unwrap();

    let mut cmd = Command::new("sd-cli");
    cmd.current_dir(&config.models_path);

    // CLI Options
    cmd.arg("--output").arg(&config.output_path);
    if config.verbose {
        cmd.arg("-v");
    }
    if config.color {
        cmd.arg("--color");
    }
    cmd.arg("--mode").arg(&config.mode);

    // Context Options
    if !config.diffusion_model.is_empty() {
        cmd.arg("--diffusion-model").arg(&config.diffusion_model);
    }
    if let Some(v) = &config.model {
        cmd.arg("--model").arg(v);
    }
    if let Some(v) = &config.clip_l {
        cmd.arg("--clip_l").arg(v);
    }
    if let Some(v) = &config.clip_g {
        cmd.arg("--clip_g").arg(v);
    }
    if let Some(v) = &config.t5xxl {
        cmd.arg("--t5xxl").arg(v);
    }
    if let Some(v) = &config.llm {
        cmd.arg("--llm").arg(v);
    }
    if let Some(v) = &config.vae {
        cmd.arg("--vae").arg(v);
    }
    if let Some(v) = &config.control_net {
        cmd.arg("--control-net").arg(v);
    }
    if let Some(v) = &config.lora_model_dir {
        cmd.arg("--lora-model-dir").arg(v);
    }

    if let Some(v) = &config.preview_path {
        cmd.arg("--preview-path").arg(v);
    }
    if let Some(v) = config.preview_interval {
        cmd.arg("--preview-interval").arg(v.to_string());
    }
    if let Some(v) = config.output_begin_idx {
        cmd.arg("--output-begin-idx").arg(v.to_string());
    }
    if config.canny {
        cmd.arg("--canny");
    }
    if let Some(v) = &config.preview_method {
        cmd.arg("--preview").arg(v);
    }

    if let Some(v) = &config.clip_vision {
        cmd.arg("--clip_vision").arg(v);
    }
    if let Some(v) = &config.llm_vision {
        cmd.arg("--llm_vision").arg(v);
    }
    if let Some(v) = &config.taesd {
        cmd.arg("--taesd").arg(v);
    }
    if let Some(v) = &config.embd_dir {
        cmd.arg("--embd-dir").arg(v);
    }
    if let Some(v) = &config.upscale_model {
        cmd.arg("--upscale-model").arg(v);
    }

    cmd.arg("--threads").arg(config.threads.to_string());
    if config.offload_to_cpu {
        cmd.arg("--offload-to-cpu");
    }
    if config.diffusion_fa {
        cmd.arg("--diffusion-fa");
    }
    cmd.arg("--rng").arg(&config.rng);

    // Generation Options
    cmd.arg("-p").arg(&config.prompt);
    cmd.arg("-n").arg(&config.negative_prompt);

    if let Some(v) = &config.init_img {
        cmd.arg("--init-img").arg(v);
    }
    if let Some(v) = &config.mask {
        cmd.arg("--mask").arg(v);
    }
    if let Some(v) = &config.control_image {
        cmd.arg("--control-image").arg(v);
    }

    cmd.arg("-H").arg(config.height.to_string());
    cmd.arg("-W").arg(config.width.to_string());
    cmd.arg("--steps").arg(config.steps.to_string());
    cmd.arg("--batch-count").arg(config.batch_count.to_string());
    cmd.arg("--cfg-scale").arg(config.cfg_scale.to_string());
    cmd.arg("--guidance").arg(config.guidance.to_string());
    cmd.arg("--strength").arg(config.strength.to_string());
    cmd.arg("--seed").arg(config.seed.to_string());
    cmd.arg("--sampling-method").arg(&config.sampling_method);
    cmd.arg("--scheduler").arg(&config.scheduler);

    // Make sure output dir exists
    let out_dir = Path::new(&config.models_path).join(
        Path::new(&config.output_path)
            .parent()
            .unwrap_or(Path::new(".")),
    );
    if !out_dir.exists() {
        let _ = std::fs::create_dir_all(&out_dir);
    }

    println!("🚀 Starting sd-cli: {:?}", cmd);

    match cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        Ok(child) => {
            *process_guard = Some(child);
            Ok(HttpResponse::Ok().json(SDServerResponse {
                success: true,
                message: "SD generation started successfully".to_string(),
            }))
        }
        Err(e) => Ok(HttpResponse::InternalServerError().json(SDServerResponse {
            success: false,
            message: format!("Failed to start sd-cli: {}", e),
        })),
    }
}
