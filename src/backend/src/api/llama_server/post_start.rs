use actix_web::{post, web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::Mutex;

use crate::api::llama_server::logs_reader::spawn_log_reader;
use crate::api::llama_server::types::{Config, LogBuffer, ProcessHandle, ServerStateHandle};
use crate::api::llama_server::websocket::WebSocketState;

#[derive(Serialize, Deserialize, Debug)]
pub struct LlamaServerResponse {
    pub success: bool,
    pub message: String,
}

/// The fully resolved `llama-server` invocation derived from the current config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlamaLaunchPlan {
    /// CLI arguments, in the order they are passed to `llama-server`.
    pub args: Vec<String>,
    /// Whether a local `--model` path won over the HuggingFace `-hf` model.
    pub using_local_model: bool,
}

/// Translates a [`Config`] into the exact argument list `llama-server` is given.
///
/// Kept free of any process spawning so the argument permutations can be
/// asserted on directly.
pub fn build_llama_launch_plan(config: &Config) -> LlamaLaunchPlan {
    let mut args: Vec<String> = Vec::new();
    let mut using_local_model = false;

    if let Some(model_path) = &config.model {
        if !model_path.trim().is_empty() {
            args.push("--model".to_string());
            args.push(model_path.clone());
            using_local_model = true;
        }
    }

    if !using_local_model && !config.hf_model.trim().is_empty() {
        args.push("-hf".to_string());
        args.push(config.hf_model.clone());
    }

    args.push("--ctx-size".to_string());
    args.push(config.ctx_size.to_string());

    // Add optional arguments
    if let Some(threads_val) = config.threads {
        args.push("--threads".to_string());
        args.push(threads_val.to_string());
    }
    if let Some(threads_batch_val) = config.threads_batch {
        args.push("--threads-batch".to_string());
        args.push(threads_batch_val.to_string());
    }
    if let Some(predict_val) = config.predict {
        args.push("--predict".to_string());
        args.push(predict_val.to_string());
    }
    if let Some(batch_size_val) = config.batch_size {
        args.push("--batch-size".to_string());
        args.push(batch_size_val.to_string());
    }
    if let Some(ubatch_size_val) = config.ubatch_size {
        args.push("--ubatch-size".to_string());
        args.push(ubatch_size_val.to_string());
    }
    if let Some(true) = config.flash_attn {
        args.push("--flash-attn".to_string());
    }
    if let Some(true) = config.mlock {
        args.push("--mlock".to_string());
    }
    if let Some(true) = config.no_mmap {
        args.push("--no-mmap".to_string());
    }
    if let Some(gpu_layers_val) = config.gpu_layers {
        args.push("--gpu-layers".to_string());
        args.push(gpu_layers_val.to_string());
    }
    if let Some(host_val) = &config.host {
        args.push("--host".to_string());
        args.push(host_val.clone());
    }
    if let Some(port_val) = config.port {
        args.push("--port".to_string());
        args.push(port_val.to_string());
    }

    LlamaLaunchPlan {
        args,
        using_local_model,
    }
}

/// Isolates the only place the real `llama-server` binary is ever executed.
///
/// Tests substitute a fake so the handler's branching can be exercised without
/// launching a multi-gigabyte model server.
pub trait LlamaServerLauncher: Send + Sync {
    fn launch(&self, args: &[String]) -> std::io::Result<Child>;
}

/// Production launcher: actually runs `llama-server`.
pub struct RealLlamaServerLauncher;

impl LlamaServerLauncher for RealLlamaServerLauncher {
    fn launch(&self, args: &[String]) -> std::io::Result<Child> {
        Command::new("llama-server")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    }
}

#[post("/api/llama-server/start")]
pub async fn post_start_llama_server(
    process: web::Data<ProcessHandle>,
    config: web::Data<Arc<Mutex<Config>>>,
    log_buffer: web::Data<LogBuffer>,
    server_state: web::Data<ServerStateHandle>,
    ws_state: web::Data<Arc<WebSocketState>>,
) -> ActixResult<HttpResponse> {
    start_llama_server_with(
        &RealLlamaServerLauncher,
        process,
        config,
        log_buffer,
        server_state,
        ws_state,
    )
    .await
}

pub async fn start_llama_server_with(
    launcher: &dyn LlamaServerLauncher,
    process: web::Data<ProcessHandle>,
    config: web::Data<Arc<Mutex<Config>>>,
    log_buffer: web::Data<LogBuffer>,
    server_state: web::Data<ServerStateHandle>,
    ws_state: web::Data<Arc<WebSocketState>>,
) -> ActixResult<HttpResponse> {
    let mut process_guard = process.lock().unwrap();

    // Check if process already exists and is running
    if let Some(ref mut child) = *process_guard {
        match child.try_wait() {
            Ok(Some(_)) => {
                // Process has exited, we can start a new one
            }
            Ok(None) => {
                // Process is still running
                return Ok(HttpResponse::Ok().json(LlamaServerResponse {
                    success: false,
                    message: "Llama server is already running".to_string(),
                }));
            }
            Err(_) => {
                // Error checking process, assume it's dead
            }
        }
    }

    // Get current config
    let config_snapshot = {
        let config_guard = config.lock().unwrap();
        config_guard.clone()
    };
    let hf_model = config_snapshot.hf_model.clone();
    let port = config_snapshot.port;

    // Resolve the CLI invocation, then hand it to the launcher.
    let plan = build_llama_launch_plan(&config_snapshot);
    let using_local_model = plan.using_local_model;

    match launcher.launch(&plan.args) {
        Ok(mut child) => {
            // Reset server state
            {
                let mut state = server_state.lock().unwrap();
                state.is_ready = false;
                state.generation = state.generation.wrapping_add(1);
            }

            // Clear log buffer
            {
                let mut buffer = log_buffer.lock().unwrap();
                buffer.clear();
            }

            // Capture stdout and stderr
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            // Spawn log readers
            if stdout.is_some() || stderr.is_some() {
                spawn_log_reader(
                    stdout,
                    stderr,
                    log_buffer.get_ref().clone(),
                    server_state.get_ref().clone(),
                    Some(ws_state.get_ref().clone()),
                    port,
                );
            }

            if !using_local_model && !hf_model.trim().is_empty() {
                use crate::api::llama_server::types::{LogEntry, LogSource};
                use std::time::{SystemTime, UNIX_EPOCH};
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let line = format!("⏳ Starting or downloading HuggingFace model: {}. Note: llama.cpp hides download progress when running in the background. Please wait, this may take several minutes if the model is not cached...", hf_model.trim());

                {
                    let mut buffer = log_buffer.lock().unwrap();
                    buffer.push_back(LogEntry {
                        timestamp,
                        line: line.clone(),
                        source: LogSource::Stdout,
                    });
                }

                use crate::api::llama_server::websocket::LogLine;
                ws_state.broadcast_log(LogLine {
                    timestamp,
                    line,
                    source: "stdout".to_string(),
                });
            }

            *process_guard = Some(child);
            println!("✅ Llama server started successfully");
            Ok(HttpResponse::Ok().json(LlamaServerResponse {
                success: true,
                message: "Llama server started successfully".to_string(),
            }))
        }
        Err(e) => {
            println!("Failed to start llama server: {}", e);
            Ok(
                HttpResponse::InternalServerError().json(LlamaServerResponse {
                    success: false,
                    message: format!("Failed to start llama server: {}", e),
                }),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::llama_server::types::ServerState;
    use std::collections::VecDeque;
    use tokio::sync::mpsc::UnboundedReceiver;

    /// Spawns a deliberately trivial, short-lived process so tests have a real
    /// [`Child`] to pass around. This is never `llama-server`: the whole point of
    /// [`LlamaServerLauncher`] is that the real model server binary is never
    /// executed by the test suite.
    fn spawn_harmless_child(seconds: &str) -> Child {
        Command::new("sleep")
            .arg(seconds)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("`sleep` must be available to run these tests")
    }

    /// A child that has already terminated, for the "previous process exited"
    /// branch. `wait` caches the exit status so later `try_wait` calls see it.
    fn exited_child() -> Child {
        let mut child = spawn_harmless_child("0");
        let _ = child.wait();
        child
    }

    enum Outcome {
        /// Hand back a harmless long-lived process instead of `llama-server`.
        Harmless,
        Failure,
    }

    /// Stand-in launcher: records the arguments it was handed and replays a
    /// canned outcome, so the handler's branches run without a real binary.
    struct FakeLauncher {
        outcome: Outcome,
        calls: Mutex<Vec<Vec<String>>>,
    }

    impl FakeLauncher {
        fn new(outcome: Outcome) -> Self {
            Self {
                outcome,
                calls: Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<Vec<String>> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl LlamaServerLauncher for FakeLauncher {
        fn launch(&self, args: &[String]) -> std::io::Result<Child> {
            self.calls.lock().unwrap().push(args.to_vec());
            match self.outcome {
                Outcome::Harmless => Ok(spawn_harmless_child("5")),
                Outcome::Failure => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "no such file or directory",
                )),
            }
        }
    }

    struct Harness {
        process: web::Data<ProcessHandle>,
        config: web::Data<Arc<Mutex<Config>>>,
        log_buffer: web::Data<LogBuffer>,
        server_state: web::Data<ServerStateHandle>,
        ws_state: web::Data<Arc<WebSocketState>>,
        rx: UnboundedReceiver<String>,
    }

    impl Harness {
        fn new(config: Config) -> Self {
            let process = ProcessHandle(Arc::new(Mutex::new(None)));
            let log_buffer: LogBuffer = Arc::new(Mutex::new(VecDeque::new()));
            let server_state: ServerStateHandle = Arc::new(Mutex::new(ServerState {
                is_ready: true,
                generation: 7,
            }));
            let ws_state = Arc::new(WebSocketState::new(
                web::Data::new(log_buffer.clone()),
                web::Data::new(process.clone()),
                web::Data::new(server_state.clone()),
            ));
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
            ws_state.add_logs_client("test-client".to_string(), tx);

            Self {
                process: web::Data::new(process),
                config: web::Data::new(Arc::new(Mutex::new(config))),
                log_buffer: web::Data::new(log_buffer),
                server_state: web::Data::new(server_state),
                ws_state: web::Data::new(ws_state),
                rx,
            }
        }

        async fn start(&self, launcher: &dyn LlamaServerLauncher) -> (u16, LlamaServerResponse) {
            let resp = start_llama_server_with(
                launcher,
                self.process.clone(),
                self.config.clone(),
                self.log_buffer.clone(),
                self.server_state.clone(),
                self.ws_state.clone(),
            )
            .await
            .unwrap();

            let status = resp.status().as_u16();
            let body = actix_web::body::to_bytes(resp.into_body()).await.unwrap();
            (status, serde_json::from_slice(&body).unwrap())
        }

        fn log_lines(&self) -> Vec<String> {
            self.log_buffer
                .lock()
                .unwrap()
                .iter()
                .map(|e| e.line.clone())
                .collect()
        }

        fn broadcasts(&mut self) -> Vec<String> {
            let mut out = Vec::new();
            while let Ok(msg) = self.rx.try_recv() {
                out.push(msg);
            }
            out
        }

        /// Nothing spawned by these tests should outlive them.
        fn kill_managed_process(&self) {
            if let Some(child) = self.process.lock().unwrap().as_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    fn full_config() -> Config {
        Config {
            hf_model: "org/model:Q4".to_string(),
            ctx_size: 4096,
            threads: Some(8),
            threads_batch: Some(4),
            predict: Some(-1),
            batch_size: Some(512),
            ubatch_size: Some(128),
            flash_attn: Some(true),
            mlock: Some(true),
            no_mmap: Some(true),
            gpu_layers: Some(35),
            model: None,
            host: Some("0.0.0.0".to_string()),
            port: Some(8099),
        }
    }

    #[test]
    fn test_launch_plan_default_config_uses_hf_model_and_ctx_size_only() {
        let plan = build_llama_launch_plan(&Config::default());

        assert!(!plan.using_local_model);
        assert_eq!(
            plan.args,
            vec![
                "-hf",
                "unsloth/DeepSeek-R1-0528-Qwen3-8B-GGUF:Q6_K_XL",
                "--ctx-size",
                "0",
            ]
        );
    }

    #[test]
    fn test_launch_plan_local_model_wins_over_hf_model() {
        let config = Config {
            model: Some("/models/local.gguf".to_string()),
            ..full_config()
        };

        let plan = build_llama_launch_plan(&config);

        assert!(plan.using_local_model);
        assert_eq!(plan.args[0], "--model");
        assert_eq!(plan.args[1], "/models/local.gguf");
        assert!(
            !plan.args.iter().any(|a| a == "-hf"),
            "a local model path must suppress the HuggingFace flag: {:?}",
            plan.args
        );
    }

    #[test]
    fn test_launch_plan_whitespace_only_model_falls_back_to_hf_model() {
        let config = Config {
            model: Some("   ".to_string()),
            hf_model: "org/model:Q4".to_string(),
            ..Config::default()
        };

        let plan = build_llama_launch_plan(&config);

        assert!(!plan.using_local_model);
        assert_eq!(plan.args, vec!["-hf", "org/model:Q4", "--ctx-size", "0"]);
    }

    #[test]
    fn test_launch_plan_blank_hf_model_and_no_local_model_yields_no_model_args() {
        let config = Config {
            hf_model: "  ".to_string(),
            model: None,
            ctx_size: 2048,
            ..Config::default()
        };

        let plan = build_llama_launch_plan(&config);

        assert_eq!(plan.args, vec!["--ctx-size", "2048"]);
    }

    #[test]
    fn test_launch_plan_emits_every_optional_argument_in_order() {
        let plan = build_llama_launch_plan(&full_config());

        assert_eq!(
            plan.args,
            vec![
                "-hf",
                "org/model:Q4",
                "--ctx-size",
                "4096",
                "--threads",
                "8",
                "--threads-batch",
                "4",
                "--predict",
                "-1",
                "--batch-size",
                "512",
                "--ubatch-size",
                "128",
                "--flash-attn",
                "--mlock",
                "--no-mmap",
                "--gpu-layers",
                "35",
                "--host",
                "0.0.0.0",
                "--port",
                "8099",
            ]
        );
    }

    #[test]
    fn test_launch_plan_omits_flags_explicitly_set_to_false() {
        let config = Config {
            flash_attn: Some(false),
            mlock: Some(false),
            no_mmap: Some(false),
            ..full_config()
        };

        let plan = build_llama_launch_plan(&config);

        for flag in ["--flash-attn", "--mlock", "--no-mmap"] {
            assert!(
                !plan.args.iter().any(|a| a == flag),
                "{} should be omitted when the option is Some(false)",
                flag
            );
        }
        // The non-boolean options are unaffected.
        assert!(plan.args.iter().any(|a| a == "--gpu-layers"));
    }

    #[actix_web::test]
    async fn test_start_refuses_when_a_process_is_still_running() {
        let harness = Harness::new(Config::default());
        *harness.process.lock().unwrap() = Some(spawn_harmless_child("5"));
        let launcher = FakeLauncher::new(Outcome::Harmless);

        let (status, body) = harness.start(&launcher).await;

        assert_eq!(status, 200);
        assert!(!body.success);
        assert_eq!(body.message, "Llama server is already running");
        assert!(
            launcher.calls().is_empty(),
            "no launch should be attempted while a process is alive"
        );
        // The live process is left untouched and the generation is not bumped.
        assert!(harness.process.lock().unwrap().is_some());
        assert_eq!(harness.server_state.lock().unwrap().generation, 7);

        harness.kill_managed_process();
    }

    #[actix_web::test]
    async fn test_start_launches_again_after_the_previous_process_exited() {
        let harness = Harness::new(Config::default());
        *harness.process.lock().unwrap() = Some(exited_child());
        harness
            .log_buffer
            .lock()
            .unwrap()
            .push_back(crate::api::llama_server::types::LogEntry {
                timestamp: 1,
                line: "stale line from the previous run".to_string(),
                source: crate::api::llama_server::types::LogSource::Stdout,
            });
        let launcher = FakeLauncher::new(Outcome::Harmless);

        let (status, body) = harness.start(&launcher).await;

        assert_eq!(status, 200);
        assert!(body.success);
        assert_eq!(body.message, "Llama server started successfully");
        assert_eq!(launcher.calls().len(), 1);

        // Server state is reset and the generation advanced so stale log
        // readers from the dead process are ignored.
        let state = harness.server_state.lock().unwrap().clone();
        assert!(!state.is_ready);
        assert_eq!(state.generation, 8);

        // The stale log line is gone; only the fresh HF notice remains.
        assert!(
            !harness
                .log_lines()
                .iter()
                .any(|l| l.contains("stale line from the previous run")),
            "the log buffer should be cleared on a successful start"
        );

        harness.kill_managed_process();
    }

    #[actix_web::test]
    async fn test_start_with_hf_model_records_and_broadcasts_a_download_notice() {
        let mut harness = Harness::new(Config {
            hf_model: "  org/slow-model:Q8  ".to_string(),
            model: None,
            ..Config::default()
        });
        let launcher = FakeLauncher::new(Outcome::Harmless);

        let (status, body) = harness.start(&launcher).await;
        assert_eq!(status, 200);
        assert!(body.success);

        let lines = harness.log_lines();
        assert_eq!(
            lines.len(),
            1,
            "expected exactly the HF notice: {:?}",
            lines
        );
        assert!(lines[0].contains("Starting or downloading HuggingFace model: org/slow-model:Q8"));
        assert!(
            !lines[0].contains("  org/"),
            "the model name should be trimmed in the notice"
        );

        let broadcasts = harness.broadcasts();
        assert_eq!(broadcasts.len(), 1);
        let parsed: serde_json::Value = serde_json::from_str(&broadcasts[0]).unwrap();
        assert_eq!(parsed["type"], "log");
        assert_eq!(parsed["log"]["source"], "stdout");
        assert!(parsed["log"]["line"]
            .as_str()
            .unwrap()
            .contains("org/slow-model:Q8"));

        harness.kill_managed_process();
    }

    #[actix_web::test]
    async fn test_start_with_local_model_skips_the_hf_download_notice() {
        let mut harness = Harness::new(Config {
            model: Some("/models/local.gguf".to_string()),
            ..Config::default()
        });
        let launcher = FakeLauncher::new(Outcome::Harmless);

        let (status, body) = harness.start(&launcher).await;

        assert_eq!(status, 200);
        assert!(body.success);
        assert_eq!(launcher.calls()[0][0], "--model");
        assert!(
            harness.log_lines().is_empty(),
            "a local model needs no download notice: {:?}",
            harness.log_lines()
        );
        assert!(harness.broadcasts().is_empty());

        harness.kill_managed_process();
    }

    #[actix_web::test]
    async fn test_start_reports_500_when_the_launcher_fails() {
        let mut harness = Harness::new(Config::default());
        let launcher = FakeLauncher::new(Outcome::Failure);

        let (status, body) = harness.start(&launcher).await;

        assert_eq!(status, 500);
        assert!(!body.success);
        assert!(
            body.message
                .starts_with("Failed to start llama server: no such file or directory"),
            "unexpected message: {}",
            body.message
        );
        assert!(
            harness.process.lock().unwrap().is_none(),
            "a failed launch must not register a process"
        );
        // A failed launch leaves the previous state and logs alone.
        assert_eq!(harness.server_state.lock().unwrap().generation, 7);
        assert!(harness.log_lines().is_empty());
        assert!(harness.broadcasts().is_empty());
    }
}
