use actix_web::{post, web, HttpResponse, Result as ActixResult};
use serde::Serialize;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::api::sd_server::types::{LogBuffer, SDConfigHandle, SDProcessHandle, SDStateHandle};
use crate::api::sd_server::websocket::WebSocketState;
use std::sync::Arc;

#[derive(Serialize, Debug)]
pub struct SDServerResponse {
    pub success: bool,
    pub message: String,
}

#[post("/api/sd-server/start")]
pub async fn post_start_sd_server(
    process: web::Data<SDProcessHandle>,
    config: web::Data<SDConfigHandle>,
    // Add missing dependencies
    log_buffer: web::Data<LogBuffer>,
    sd_state: web::Data<SDStateHandle>,
    ws_state: web::Data<Arc<WebSocketState>>,
    sd_images_storage: web::Data<Arc<crate::api::sd_server::storage::SDImagesStorage>>,
) -> ActixResult<HttpResponse> {
    let mut process_guard = process.lock().unwrap();

    if let Some(ref mut child) = *process_guard {
        // Explicitly annotate type for inference
        let status: std::io::Result<Option<std::process::ExitStatus>> = child.try_wait();
        match status {
            Ok(Some(_)) => {} // Process finished
            Ok(None) => {
                return Ok(HttpResponse::Ok().json(SDServerResponse {
                    success: false,
                    message: "SD generation is already running".to_string(),
                }));
            }
            Err(_) => {} // Error checking, assume we can try starting
        }
    }

    let config = config.lock().unwrap();

    let mut cmd = Command::new("sd-cli");
    cmd.current_dir(&config.models_path);

    // Resolve absolute path for output to ensure it goes to the correct directory
    let output_path_abs = if Path::new(&config.output_path).is_absolute() {
        config.output_path.clone()
    } else {
        std::env::current_dir()
            .unwrap()
            .join(&config.output_path)
            .to_string_lossy()
            .to_string()
    };

    // CLI Options
    // Generate unique filename
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let unique_filename = format!("output_{}.png", timestamp);
    let output_file_abs = std::path::Path::new(&output_path_abs).join(&unique_filename);

    // CLI Options
    cmd.arg("--output")
        .arg(output_file_abs.to_string_lossy().to_string());

    if config.verbose {
        cmd.arg("-v");
    }
    if config.color {
        cmd.arg("--color");
    }

    // Only add mode if set and not empty
    if let Some(v) = &config.mode {
        if !v.is_empty() {
            cmd.arg("--mode").arg(v);
        }
    }

    // Context Options
    if !config.diffusion_model.is_empty() {
        cmd.arg("--diffusion-model").arg(&config.diffusion_model);
    }
    if let Some(v) = &config.model {
        if !v.is_empty() {
            cmd.arg("--model").arg(v);
        }
    }
    if let Some(v) = &config.clip_l {
        if !v.is_empty() {
            cmd.arg("--clip_l").arg(v);
        }
    }
    if let Some(v) = &config.clip_g {
        if !v.is_empty() {
            cmd.arg("--clip_g").arg(v);
        }
    }
    if let Some(v) = &config.t5xxl {
        if !v.is_empty() {
            cmd.arg("--t5xxl").arg(v);
        }
    }
    if let Some(v) = &config.llm {
        if !v.is_empty() {
            cmd.arg("--llm").arg(v);
        }
    }
    if let Some(v) = &config.vae {
        if !v.is_empty() {
            cmd.arg("--vae").arg(v);
        }
    }
    if let Some(v) = &config.control_net {
        if !v.is_empty() {
            cmd.arg("--control-net").arg(v);
        }
    }
    if let Some(v) = &config.lora_model_dir {
        if !v.is_empty() {
            cmd.arg("--lora-model-dir").arg(v);
        }
    }

    if let Some(v) = &config.preview_path {
        if !v.is_empty() {
            cmd.arg("--preview-path").arg(v);
        }
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
        if !v.is_empty() {
            cmd.arg("--preview").arg(v);
        }
    }

    if let Some(v) = &config.clip_vision {
        if !v.is_empty() {
            cmd.arg("--clip_vision").arg(v);
        }
    }
    if let Some(v) = &config.llm_vision {
        if !v.is_empty() {
            cmd.arg("--llm_vision").arg(v);
        }
    }
    if let Some(v) = &config.taesd {
        if !v.is_empty() {
            cmd.arg("--taesd").arg(v);
        }
    }
    if let Some(v) = &config.embd_dir {
        if !v.is_empty() {
            cmd.arg("--embd-dir").arg(v);
        }
    }
    if let Some(v) = &config.upscale_model {
        if !v.is_empty() {
            cmd.arg("--upscale-model").arg(v);
        }
    }

    // Only set threads if not -1 (auto)
    if config.threads != -1 {
        cmd.arg("--threads").arg(config.threads.to_string());
    }
    if config.offload_to_cpu {
        cmd.arg("--offload-to-cpu");
    }
    if config.diffusion_fa {
        cmd.arg("--diffusion-fa");
    }
    if config.control_net_cpu {
        cmd.arg("--control-net-cpu");
    }
    if config.clip_on_cpu {
        cmd.arg("--clip-on-cpu");
    }
    if config.vae_on_cpu {
        cmd.arg("--vae-on-cpu");
    }
    if config.vae_tiling {
        cmd.arg("--vae-tiling");
    }
    if let Some(v) = config.vae_tile_size {
        cmd.arg("--vae-tile-size").arg(v.to_string());
    }
    if let Some(v) = config.vae_relative_tile_size {
        cmd.arg("--vae-relative-tile-size").arg(v.to_string());
    }
    // Only set RNG if not standard default
    if config.rng != "std_default" && !config.rng.is_empty() {
        cmd.arg("--rng").arg(&config.rng);
    }

    // Generation Options
    if !config.prompt.is_empty() {
        cmd.arg("-p").arg(&config.prompt);
    }
    if !config.negative_prompt.is_empty() {
        cmd.arg("-n").arg(&config.negative_prompt);
    }

    if let Some(v) = &config.init_img {
        if !v.is_empty() {
            cmd.arg("--init-img").arg(v);
        }
    }
    if let Some(v) = &config.mask {
        if !v.is_empty() {
            cmd.arg("--mask").arg(v);
        }
    }
    if let Some(v) = &config.control_image {
        if !v.is_empty() {
            cmd.arg("--control-image").arg(v);
        }
    }

    cmd.arg("-H").arg(config.height.to_string());
    cmd.arg("-W").arg(config.width.to_string());

    // cfg-scale is f32, required
    cmd.arg("--cfg-scale").arg(config.cfg_scale.to_string());

    if let Some(v) = config.steps {
        cmd.arg("--steps").arg(v.to_string());
    }
    if let Some(v) = config.batch_count {
        cmd.arg("--batch-count").arg(v.to_string());
    }

    if let Some(v) = config.guidance {
        cmd.arg("--guidance").arg(v.to_string());
    }
    if let Some(v) = config.strength {
        cmd.arg("--strength").arg(v.to_string());
    }
    if let Some(v) = config.seed {
        cmd.arg("--seed").arg(v.to_string());
    }
    if let Some(v) = &config.sampling_method {
        if !v.is_empty() {
            cmd.arg("--sampling-method").arg(v);
        }
    }
    if let Some(v) = &config.scheduler {
        if !v.is_empty() {
            cmd.arg("--scheduler").arg(v);
        }
    }

    // Make sure output dir exists
    let out_dir = Path::new(&output_path_abs);

    if !out_dir.exists() {
        let _ = std::fs::create_dir_all(out_dir);
    }

    // Update state to generating
    {
        let mut s = sd_state.lock().unwrap();
        s.is_generating = true;
        s.current_output_file = None;
        s.pending_filename = Some(unique_filename.clone());
    }
    // Broadcast start status
    ws_state.broadcast_status(true, None);

    println!("🚀 Starting sd-cli: {:?}", cmd);

    // Save Metadata to DB
    // Construct additional info
    #[derive(Serialize)]
    struct AdditionalInfo {
        vae: Option<String>,
        control_net: Option<String>,
        lora_model_dir: Option<String>,
        t5xxl: Option<String>,
        clip_l: Option<String>,
        clip_g: Option<String>,
    }
    let additional_info = AdditionalInfo {
        vae: config.vae.clone(),
        control_net: config.control_net.clone(),
        lora_model_dir: config.lora_model_dir.clone(),
        t5xxl: config.t5xxl.clone(),
        clip_l: config.clip_l.clone(),
        clip_g: config.clip_g.clone(),
    };

    use crate::api::sd_server::storage::SDImageMetadata;
    let metadata = SDImageMetadata {
        filename: unique_filename.clone(),
        prompt: config.prompt.clone(),
        diffusion_model: config.diffusion_model.clone(),
        width: config.width as i64,
        height: config.height as i64,
        steps: config.steps.map(|v| v as i64),
        cfg_scale: config.cfg_scale,
        seed: config.seed, // stored as i64
        created_at: timestamp as i64,
        additional_info: Some(serde_json::to_string(&additional_info).unwrap_or_default()),
    };

    let storage_clone = sd_images_storage.clone();
    actix_rt::spawn(async move {
        if let Err(e) = storage_clone.add_image(metadata).await {
            println!("❌ Failed to save image metadata: {:?}", e);
        }
    });

    // FIX: Spawn logger
    match cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        Ok(mut child) => {
            // Spawn log reader
            use crate::api::sd_server::logs_reader::spawn_log_reader;

            spawn_log_reader(
                child.stdout.take(),
                child.stderr.take(),
                log_buffer.get_ref().clone(),
                sd_state.get_ref().clone(),
                (*ws_state.get_ref()).clone(),
                (*sd_images_storage.get_ref()).clone(),
            );

            *process_guard = Some(child);

            Ok(HttpResponse::Ok().json(SDServerResponse {
                success: true,
                message: "SD generation started successfully".to_string(),
            }))
        }
        Err(e) => {
            // Reset state on failure
            {
                let mut s = sd_state.lock().unwrap();
                s.is_generating = false;
            }
            ws_state.broadcast_status(false, None);

            Ok(HttpResponse::InternalServerError().json(SDServerResponse {
                success: false,
                message: format!("Failed to start sd-cli: {}", e),
            }))
        }
    }
}
