use actix_web::{post, web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use crate::api::sd_server::types::{
    LogBuffer, SDConfig, SDConfigHandle, SDProcessHandle, SDStateHandle,
};
use crate::api::sd_server::websocket::WebSocketState;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug)]
pub struct SDServerResponse {
    pub success: bool,
    pub message: String,
}

/// Resolves the configured output directory against the process working
/// directory so generated images always land where the API expects them.
pub fn resolve_output_dir(output_path: &str) -> String {
    if Path::new(output_path).is_absolute() {
        output_path.to_string()
    } else {
        std::env::current_dir()
            .unwrap()
            .join(output_path)
            .to_string_lossy()
            .to_string()
    }
}

/// Translates an [`SDConfig`] into the exact argument list `sd-cli` is given.
///
/// Deliberately free of process spawning and filesystem writes so every
/// permutation of the config can be asserted on directly.
pub fn build_sd_args(config: &SDConfig, output_file: &str) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    /// Pushes `flag value` only when the optional string is present and non-empty.
    fn push_opt(args: &mut Vec<String>, flag: &str, value: &Option<String>) {
        if let Some(v) = value {
            if !v.is_empty() {
                args.push(flag.to_string());
                args.push(v.clone());
            }
        }
    }

    // CLI Options
    args.push("--output".to_string());
    args.push(output_file.to_string());

    if config.verbose {
        args.push("-v".to_string());
    }
    if config.color {
        args.push("--color".to_string());
    }

    // Only add mode if set and not empty
    push_opt(&mut args, "--mode", &config.mode);

    // Context Options
    if !config.diffusion_model.is_empty() {
        args.push("--diffusion-model".to_string());
        args.push(config.diffusion_model.clone());
    }
    push_opt(&mut args, "--model", &config.model);
    push_opt(&mut args, "--clip_l", &config.clip_l);
    push_opt(&mut args, "--clip_g", &config.clip_g);
    push_opt(&mut args, "--t5xxl", &config.t5xxl);
    push_opt(&mut args, "--llm", &config.llm);
    push_opt(&mut args, "--vae", &config.vae);
    push_opt(&mut args, "--control-net", &config.control_net);
    push_opt(&mut args, "--lora-model-dir", &config.lora_model_dir);

    push_opt(&mut args, "--preview-path", &config.preview_path);
    if let Some(v) = config.preview_interval {
        args.push("--preview-interval".to_string());
        args.push(v.to_string());
    }
    if let Some(v) = config.output_begin_idx {
        args.push("--output-begin-idx".to_string());
        args.push(v.to_string());
    }
    if config.canny {
        args.push("--canny".to_string());
    }
    push_opt(&mut args, "--preview", &config.preview_method);

    push_opt(&mut args, "--clip_vision", &config.clip_vision);
    push_opt(&mut args, "--llm_vision", &config.llm_vision);
    push_opt(&mut args, "--taesd", &config.taesd);
    push_opt(&mut args, "--embd-dir", &config.embd_dir);
    push_opt(&mut args, "--upscale-model", &config.upscale_model);

    // Only set threads if not -1 (auto)
    if config.threads != -1 {
        args.push("--threads".to_string());
        args.push(config.threads.to_string());
    }
    if config.offload_to_cpu {
        args.push("--offload-to-cpu".to_string());
    }
    if config.diffusion_fa {
        args.push("--diffusion-fa".to_string());
    }
    if config.control_net_cpu {
        args.push("--control-net-cpu".to_string());
    }
    if config.clip_on_cpu {
        args.push("--clip-on-cpu".to_string());
    }
    if config.vae_on_cpu {
        args.push("--vae-on-cpu".to_string());
    }
    if config.vae_tiling {
        args.push("--vae-tiling".to_string());
    }
    if let Some(v) = config.vae_tile_size {
        args.push("--vae-tile-size".to_string());
        args.push(v.to_string());
    }
    if let Some(v) = config.vae_relative_tile_size {
        args.push("--vae-relative-tile-size".to_string());
        args.push(v.to_string());
    }
    // Only set RNG if not standard default
    if config.rng != "std_default" && !config.rng.is_empty() {
        args.push("--rng".to_string());
        args.push(config.rng.clone());
    }

    // Generation Options
    if !config.prompt.is_empty() {
        args.push("-p".to_string());
        args.push(config.prompt.clone());
    }
    if !config.negative_prompt.is_empty() {
        args.push("-n".to_string());
        args.push(config.negative_prompt.clone());
    }

    push_opt(&mut args, "--init-img", &config.init_img);
    push_opt(&mut args, "--mask", &config.mask);
    push_opt(&mut args, "--control-image", &config.control_image);

    args.push("-H".to_string());
    args.push(config.height.to_string());
    args.push("-W".to_string());
    args.push(config.width.to_string());

    // cfg-scale is f32, required
    args.push("--cfg-scale".to_string());
    args.push(config.cfg_scale.to_string());

    if let Some(v) = config.steps {
        args.push("--steps".to_string());
        args.push(v.to_string());
    }
    if let Some(v) = config.batch_count {
        args.push("--batch-count".to_string());
        args.push(v.to_string());
    }

    if let Some(v) = config.guidance {
        args.push("--guidance".to_string());
        args.push(v.to_string());
    }
    if let Some(v) = config.strength {
        args.push("--strength".to_string());
        args.push(v.to_string());
    }
    if let Some(v) = config.seed {
        args.push("--seed".to_string());
        args.push(v.to_string());
    }
    push_opt(&mut args, "--sampling-method", &config.sampling_method);
    push_opt(&mut args, "--scheduler", &config.scheduler);

    args
}

/// Isolates the only place the real `sd-cli` binary is ever executed.
///
/// Tests substitute a fake so the handler's branches run without invoking a
/// real image-generation backend.
pub trait SdCliLauncher: Send + Sync {
    fn launch(&self, working_dir: &str, args: &[String]) -> std::io::Result<Child>;
}

/// Production launcher: actually runs `sd-cli`.
pub struct RealSdCliLauncher;

impl SdCliLauncher for RealSdCliLauncher {
    fn launch(&self, working_dir: &str, args: &[String]) -> std::io::Result<Child> {
        Command::new("sd-cli")
            .current_dir(working_dir)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    }
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
    start_sd_server_with(
        &RealSdCliLauncher,
        process,
        config,
        log_buffer,
        sd_state,
        ws_state,
        sd_images_storage,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn start_sd_server_with(
    launcher: &dyn SdCliLauncher,
    process: web::Data<SDProcessHandle>,
    config: web::Data<SDConfigHandle>,
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

    // Resolve absolute path for output to ensure it goes to the correct directory
    let output_path_abs = resolve_output_dir(&config.output_path);

    // Generate unique filename
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let unique_filename = format!("output_{}.png", timestamp);
    let output_file_abs = std::path::Path::new(&output_path_abs).join(&unique_filename);

    let working_dir = config.models_path.clone();
    let args = build_sd_args(&config, &output_file_abs.to_string_lossy());

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

    println!("🚀 Starting sd-cli in {}: {:?}", working_dir, args);

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
    match launcher.launch(&working_dir, &args) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::sd_server::storage::{new_file_test_storage, SDImagesStorage};
    use crate::api::sd_server::types::SDState;
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use tokio::sync::mpsc::UnboundedReceiver;

    /// Spawns a deliberately trivial, short-lived process so tests have a real
    /// [`Child`] to pass around. This is never `sd-cli`: the point of
    /// [`SdCliLauncher`] is that the real image-generation binary is never
    /// executed by the test suite.
    fn spawn_harmless_child(seconds: &str) -> Child {
        Command::new("sleep")
            .arg(seconds)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("`sleep` must be available to run these tests")
    }

    fn exited_child() -> Child {
        let mut child = spawn_harmless_child("0");
        let _ = child.wait();
        child
    }

    enum Outcome {
        /// Hand back a harmless long-lived process instead of `sd-cli`.
        Harmless,
        Failure,
    }

    struct FakeLauncher {
        outcome: Outcome,
        calls: Mutex<Vec<(String, Vec<String>)>>,
    }

    impl FakeLauncher {
        fn new(outcome: Outcome) -> Self {
            Self {
                outcome,
                calls: Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<(String, Vec<String>)> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl SdCliLauncher for FakeLauncher {
        fn launch(&self, working_dir: &str, args: &[String]) -> std::io::Result<Child> {
            self.calls
                .lock()
                .unwrap()
                .push((working_dir.to_string(), args.to_vec()));
            match self.outcome {
                Outcome::Harmless => Ok(spawn_harmless_child("5")),
                Outcome::Failure => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "no such file or directory",
                )),
            }
        }
    }

    /// Everything the handler needs, wired to throwaway directories and a
    /// throwaway database, plus a WebSocket receiver to assert broadcasts on.
    struct Harness {
        process: web::Data<SDProcessHandle>,
        config: web::Data<SDConfigHandle>,
        log_buffer: web::Data<LogBuffer>,
        sd_state: web::Data<SDStateHandle>,
        ws_state: web::Data<Arc<WebSocketState>>,
        storage: web::Data<Arc<SDImagesStorage>>,
        rx: UnboundedReceiver<String>,
        output_dir: std::path::PathBuf,
        _db_dir: tempfile::TempDir,
        _work_dir: tempfile::TempDir,
    }

    async fn harness() -> Harness {
        let work_dir = tempfile::tempdir().unwrap();
        // A not-yet-existing subdirectory, so the handler has to create it.
        let output_dir = work_dir.path().join("generated");

        let config: SDConfigHandle = Arc::new(Mutex::new(SDConfig {
            output_path: output_dir.to_string_lossy().to_string(),
            models_path: work_dir.path().to_string_lossy().to_string(),
            ..SDConfig::default()
        }));
        let process: SDProcessHandle = Arc::new(Mutex::new(None));
        let log_buffer: LogBuffer = Arc::new(Mutex::new(VecDeque::new()));
        let sd_state: SDStateHandle = Arc::new(Mutex::new(SDState {
            is_generating: false,
            current_output_file: Some("stale.png".to_string()),
            pending_filename: None,
        }));

        let ws_state = Arc::new(WebSocketState::new(
            web::Data::new(log_buffer.clone()),
            web::Data::new(process.clone()),
            web::Data::new(config.clone()),
            web::Data::new(sd_state.clone()),
        ));
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        ws_state.add_client("test-client".to_string(), tx);

        let (db_dir, storage) = new_file_test_storage().await;

        Harness {
            process: web::Data::new(process),
            config: web::Data::new(config),
            log_buffer: web::Data::new(log_buffer),
            sd_state: web::Data::new(sd_state),
            ws_state: web::Data::new(ws_state),
            storage: web::Data::new(Arc::new(storage)),
            rx,
            output_dir,
            _db_dir: db_dir,
            _work_dir: work_dir,
        }
    }

    impl Harness {
        async fn start(&self, launcher: &dyn SdCliLauncher) -> (u16, SDServerResponse) {
            let resp = start_sd_server_with(
                launcher,
                self.process.clone(),
                self.config.clone(),
                self.log_buffer.clone(),
                self.sd_state.clone(),
                self.ws_state.clone(),
                self.storage.clone(),
            )
            .await
            .unwrap();

            let status = resp.status().as_u16();
            let body = actix_web::body::to_bytes(resp.into_body()).await.unwrap();
            (status, serde_json::from_slice(&body).unwrap())
        }

        fn broadcasts(&mut self) -> Vec<serde_json::Value> {
            let mut out = Vec::new();
            while let Ok(msg) = self.rx.try_recv() {
                out.push(serde_json::from_str(&msg).unwrap());
            }
            out
        }

        /// The metadata insert happens on a spawned task, so give it a bounded
        /// window to land.
        async fn wait_for_one_image(&self) -> Vec<crate::api::sd_server::storage::SDImageMetadata> {
            for _ in 0..100 {
                let images = self.storage.get_images().await.unwrap();
                if !images.is_empty() {
                    return images;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
            panic!("metadata row was never written");
        }

        fn kill_managed_process(&self) {
            if let Some(child) = self.process.lock().unwrap().as_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    /// Every optional field populated, so each conditional argument is emitted.
    fn maximal_config() -> SDConfig {
        SDConfig {
            output_path: "/tmp/out".to_string(),
            preview_path: Some("/tmp/preview".to_string()),
            preview_interval: Some(5),
            output_begin_idx: Some(2),
            canny: true,
            verbose: true,
            color: true,
            mode: Some("img_gen".to_string()),
            preview_method: Some("tae".to_string()),
            diffusion_model: "flux.gguf".to_string(),
            model: Some("full.safetensors".to_string()),
            clip_l: Some("clip_l.gguf".to_string()),
            clip_g: Some("clip_g.gguf".to_string()),
            clip_vision: Some("clipvis.gguf".to_string()),
            t5xxl: Some("t5.gguf".to_string()),
            llm: Some("qwen.gguf".to_string()),
            llm_vision: Some("qwenvis.gguf".to_string()),
            vae: Some("ae.safetensors".to_string()),
            taesd: Some("taesd.gguf".to_string()),
            control_net: Some("cn.gguf".to_string()),
            embd_dir: Some("/embd".to_string()),
            lora_model_dir: Some("/loras".to_string()),
            upscale_model: Some("upscale.gguf".to_string()),
            threads: 12,
            offload_to_cpu: true,
            diffusion_fa: true,
            control_net_cpu: true,
            clip_on_cpu: true,
            vae_on_cpu: true,
            vae_tiling: true,
            vae_tile_size: Some(512),
            vae_relative_tile_size: Some(0.5),
            models_path: "/models".to_string(),
            rng: "cuda".to_string(),
            prompt: "a fox".to_string(),
            negative_prompt: "blurry".to_string(),
            init_img: Some("init.png".to_string()),
            mask: Some("mask.png".to_string()),
            control_image: Some("ctrl.png".to_string()),
            height: 768,
            width: 512,
            steps: Some(20),
            batch_count: Some(3),
            cfg_scale: 4.5,
            guidance: Some(3.5),
            strength: Some(0.75),
            seed: Some(42),
            sampling_method: Some("euler".to_string()),
            scheduler: Some("karras".to_string()),
        }
    }

    /// Every optional field left unset and every boolean off.
    fn minimal_config() -> SDConfig {
        SDConfig {
            canny: false,
            verbose: false,
            color: false,
            diffusion_model: String::new(),
            threads: -1,
            offload_to_cpu: false,
            diffusion_fa: false,
            control_net_cpu: false,
            clip_on_cpu: false,
            vae_on_cpu: false,
            vae_tiling: false,
            rng: "std_default".to_string(),
            prompt: String::new(),
            negative_prompt: String::new(),
            llm: None,
            vae: None,
            ..SDConfig::default()
        }
    }

    #[test]
    fn test_resolve_output_dir_keeps_an_absolute_path_untouched() {
        // A Unix-style literal like "/var/tmp/images" is not `Path::is_absolute()`
        // on Windows (which requires a drive letter/UNC prefix), so build an
        // absolute path from the current directory instead of hardcoding one -
        // this is absolute on every platform the tests run on.
        let already_absolute = std::env::current_dir().unwrap().join("images");
        let input = already_absolute.to_string_lossy().to_string();
        assert_eq!(resolve_output_dir(&input), input);
    }

    #[test]
    fn test_resolve_output_dir_anchors_a_relative_path_to_the_working_directory() {
        let resolved = resolve_output_dir("./public");
        let expected = std::env::current_dir().unwrap().join("./public");

        assert_eq!(resolved, expected.to_string_lossy());
        assert!(Path::new(&resolved).is_absolute());
    }

    #[test]
    fn test_build_sd_args_minimal_config_only_emits_the_required_arguments() {
        let args = build_sd_args(&minimal_config(), "/out/a.png");

        assert_eq!(
            args,
            vec![
                "--output",
                "/out/a.png",
                "-H",
                "1024",
                "-W",
                "1024",
                "--cfg-scale",
                "1",
            ]
        );
    }

    #[test]
    fn test_build_sd_args_maximal_config_emits_every_argument_in_order() {
        let args = build_sd_args(&maximal_config(), "/out/a.png");

        assert_eq!(
            args,
            vec![
                "--output",
                "/out/a.png",
                "-v",
                "--color",
                "--mode",
                "img_gen",
                "--diffusion-model",
                "flux.gguf",
                "--model",
                "full.safetensors",
                "--clip_l",
                "clip_l.gguf",
                "--clip_g",
                "clip_g.gguf",
                "--t5xxl",
                "t5.gguf",
                "--llm",
                "qwen.gguf",
                "--vae",
                "ae.safetensors",
                "--control-net",
                "cn.gguf",
                "--lora-model-dir",
                "/loras",
                "--preview-path",
                "/tmp/preview",
                "--preview-interval",
                "5",
                "--output-begin-idx",
                "2",
                "--canny",
                "--preview",
                "tae",
                "--clip_vision",
                "clipvis.gguf",
                "--llm_vision",
                "qwenvis.gguf",
                "--taesd",
                "taesd.gguf",
                "--embd-dir",
                "/embd",
                "--upscale-model",
                "upscale.gguf",
                "--threads",
                "12",
                "--offload-to-cpu",
                "--diffusion-fa",
                "--control-net-cpu",
                "--clip-on-cpu",
                "--vae-on-cpu",
                "--vae-tiling",
                "--vae-tile-size",
                "512",
                "--vae-relative-tile-size",
                "0.5",
                "--rng",
                "cuda",
                "-p",
                "a fox",
                "-n",
                "blurry",
                "--init-img",
                "init.png",
                "--mask",
                "mask.png",
                "--control-image",
                "ctrl.png",
                "-H",
                "768",
                "-W",
                "512",
                "--cfg-scale",
                "4.5",
                "--steps",
                "20",
                "--batch-count",
                "3",
                "--guidance",
                "3.5",
                "--strength",
                "0.75",
                "--seed",
                "42",
                "--sampling-method",
                "euler",
                "--scheduler",
                "karras",
            ]
        );
    }

    #[test]
    fn test_build_sd_args_treats_empty_optional_strings_as_unset() {
        let config = SDConfig {
            mode: Some(String::new()),
            model: Some(String::new()),
            clip_l: Some(String::new()),
            clip_g: Some(String::new()),
            t5xxl: Some(String::new()),
            llm: Some(String::new()),
            vae: Some(String::new()),
            control_net: Some(String::new()),
            lora_model_dir: Some(String::new()),
            preview_path: Some(String::new()),
            preview_method: Some(String::new()),
            clip_vision: Some(String::new()),
            llm_vision: Some(String::new()),
            taesd: Some(String::new()),
            embd_dir: Some(String::new()),
            upscale_model: Some(String::new()),
            init_img: Some(String::new()),
            mask: Some(String::new()),
            control_image: Some(String::new()),
            sampling_method: Some(String::new()),
            scheduler: Some(String::new()),
            ..minimal_config()
        };

        let args = build_sd_args(&config, "/out/a.png");

        assert_eq!(args, build_sd_args(&minimal_config(), "/out/a.png"));
    }

    #[test]
    fn test_build_sd_args_omits_auto_threads_and_default_rng() {
        let auto = SDConfig {
            threads: -1,
            rng: "std_default".to_string(),
            ..maximal_config()
        };
        let explicit = SDConfig {
            threads: 0,
            rng: "cpu".to_string(),
            ..maximal_config()
        };

        let auto_args = build_sd_args(&auto, "/out/a.png");
        assert!(!auto_args.iter().any(|a| a == "--threads"));
        assert!(!auto_args.iter().any(|a| a == "--rng"));

        let explicit_args = build_sd_args(&explicit, "/out/a.png");
        let threads_idx = explicit_args.iter().position(|a| a == "--threads").unwrap();
        assert_eq!(explicit_args[threads_idx + 1], "0");
        let rng_idx = explicit_args.iter().position(|a| a == "--rng").unwrap();
        assert_eq!(explicit_args[rng_idx + 1], "cpu");
    }

    #[test]
    fn test_build_sd_args_omits_rng_when_blank() {
        let config = SDConfig {
            rng: String::new(),
            ..minimal_config()
        };

        assert!(!build_sd_args(&config, "/out/a.png")
            .iter()
            .any(|a| a == "--rng"));
    }

    #[actix_web::test]
    async fn test_start_refuses_while_a_generation_is_still_running() {
        let mut h = harness().await;
        *h.process.lock().unwrap() = Some(spawn_harmless_child("5"));
        let launcher = FakeLauncher::new(Outcome::Harmless);

        let (status, body) = h.start(&launcher).await;

        assert_eq!(status, 200);
        assert!(!body.success);
        assert_eq!(body.message, "SD generation is already running");
        assert!(
            launcher.calls().is_empty(),
            "no launch should be attempted while a generation is alive"
        );
        // Nothing about the run state or the clients is touched.
        assert!(!h.sd_state.lock().unwrap().is_generating);
        assert!(h.broadcasts().is_empty());

        h.kill_managed_process();
    }

    #[actix_web::test]
    async fn test_start_launches_records_metadata_and_marks_state_generating() {
        let mut h = harness().await;
        // A previous, already-finished generation must not block a new one.
        *h.process.lock().unwrap() = Some(exited_child());
        let launcher = FakeLauncher::new(Outcome::Harmless);

        let (status, body) = h.start(&launcher).await;

        assert_eq!(status, 200);
        assert!(body.success);
        assert_eq!(body.message, "SD generation started successfully");

        // The launcher was invoked in the configured models directory with the
        // args derived from the config.
        let calls = launcher.calls();
        assert_eq!(calls.len(), 1);
        let (working_dir, args) = &calls[0];
        assert_eq!(working_dir, &h.config.lock().unwrap().models_path);
        let output_idx = args.iter().position(|a| a == "--output").unwrap();
        assert!(args[output_idx + 1].starts_with(&h.output_dir.to_string_lossy().to_string()));

        // The output directory is created on demand.
        assert!(h.output_dir.is_dir(), "output directory should be created");

        // Run state flips to generating, with the pending filename recorded and
        // any stale output file cleared.
        let pending = {
            let state = h.sd_state.lock().unwrap();
            assert!(state.is_generating);
            assert!(state.current_output_file.is_none());
            state.pending_filename.clone().unwrap()
        };
        assert!(pending.starts_with("output_") && pending.ends_with(".png"));
        assert_eq!(
            args[output_idx + 1],
            h.output_dir.join(&pending).to_string_lossy()
        );

        // Clients are told a generation began.
        let broadcasts = h.broadcasts();
        assert_eq!(broadcasts.len(), 1);
        assert_eq!(broadcasts[0]["type"], "status");
        assert_eq!(broadcasts[0]["is_generating"], true);
        assert_eq!(broadcasts[0]["current_file"], serde_json::Value::Null);

        // Metadata for the pending image is persisted.
        let images = h.wait_for_one_image().await;
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].filename, pending);
        assert_eq!(images[0].prompt, "A beautiful landscape");
        assert_eq!(images[0].diffusion_model, "z_image_turbo-Q8_0.gguf");
        assert_eq!(images[0].width, 1024);
        assert_eq!(images[0].height, 1024);
        let extra: serde_json::Value =
            serde_json::from_str(images[0].additional_info.as_deref().unwrap()).unwrap();
        assert_eq!(extra["vae"], "ae.safetensors");

        h.kill_managed_process();
    }

    #[actix_web::test]
    async fn test_start_resets_state_and_reports_500_when_the_launcher_fails() {
        let mut h = harness().await;
        let launcher = FakeLauncher::new(Outcome::Failure);

        let (status, body) = h.start(&launcher).await;

        assert_eq!(status, 500);
        assert!(!body.success);
        assert_eq!(
            body.message,
            "Failed to start sd-cli: no such file or directory"
        );
        assert!(
            h.process.lock().unwrap().is_none(),
            "a failed launch must not register a process"
        );
        assert!(
            !h.sd_state.lock().unwrap().is_generating,
            "the generating flag must be rolled back"
        );

        // Clients see the optimistic "started" broadcast followed by the rollback.
        let broadcasts = h.broadcasts();
        assert_eq!(broadcasts.len(), 2);
        assert_eq!(broadcasts[0]["is_generating"], true);
        assert_eq!(broadcasts[1]["is_generating"], false);

        // Documents current behaviour rather than endorsing it: the metadata row
        // is written before the launch is attempted and is *not* rolled back, so
        // a failed launch leaves a row describing an image that never exists.
        let orphans = h.wait_for_one_image().await;
        assert_eq!(orphans.len(), 1);
        assert!(!h.output_dir.join(&orphans[0].filename).exists());
    }
}
