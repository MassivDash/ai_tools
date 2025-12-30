use actix_web::{post, web, HttpResponse, Result as ActixResult};
use serde::Serialize;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::Mutex;

use crate::api::llama_server::logs_reader::spawn_log_reader;
use crate::api::llama_server::types::{Config, LogBuffer, ProcessHandle, ServerStateHandle};
use crate::api::llama_server::websocket::WebSocketState;

#[derive(Serialize, Debug)]
pub struct LlamaServerResponse {
    pub success: bool,
    pub message: String,
}

#[post("/api/llama-server/start")]
pub async fn post_start_llama_server(
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
    let config_guard = config.lock().unwrap();
    let hf_model = config_guard.hf_model.clone();
    let ctx_size = config_guard.ctx_size.to_string();
    let threads = config_guard.threads;
    let threads_batch = config_guard.threads_batch;
    let predict = config_guard.predict;
    let batch_size = config_guard.batch_size;
    let ubatch_size = config_guard.ubatch_size;
    let flash_attn = config_guard.flash_attn;
    let mlock = config_guard.mlock;
    let no_mmap = config_guard.no_mmap;
    let gpu_layers = config_guard.gpu_layers;
    let model = config_guard.model.clone();
    let host = config_guard.host.clone();
    let port = config_guard.port;
    drop(config_guard);

    // Start the llama-server process
    println!(
        "🚀 Starting llama-server with model: {}, ctx_size: {}",
        hf_model, ctx_size
    );
    let mut cmd = Command::new("llama-server");
    cmd.arg("-hf").arg(&hf_model);
    cmd.arg("--ctx-size").arg(&ctx_size);

    // Add optional arguments
    if let Some(threads_val) = threads {
        cmd.arg("--threads").arg(threads_val.to_string());
    }
    if let Some(threads_batch_val) = threads_batch {
        cmd.arg("--threads-batch")
            .arg(threads_batch_val.to_string());
    }
    if let Some(predict_val) = predict {
        cmd.arg("--predict").arg(predict_val.to_string());
    }
    if let Some(batch_size_val) = batch_size {
        cmd.arg("--batch-size").arg(batch_size_val.to_string());
    }
    if let Some(ubatch_size_val) = ubatch_size {
        cmd.arg("--ubatch-size").arg(ubatch_size_val.to_string());
    }
    if let Some(true) = flash_attn {
        cmd.arg("--flash-attn");
    }
    if let Some(true) = mlock {
        cmd.arg("--mlock");
    }
    if let Some(true) = no_mmap {
        cmd.arg("--no-mmap");
    }
    if let Some(gpu_layers_val) = gpu_layers {
        cmd.arg("--gpu-layers").arg(gpu_layers_val.to_string());
    }
    if let Some(model_path) = &model {
        if !model_path.trim().is_empty() {
            cmd.arg("--model").arg(model_path);
        }
    }
    if let Some(host_val) = &host {
        cmd.arg("--host").arg(host_val);
    }
    if let Some(port_val) = port {
        cmd.arg("--port").arg(port_val.to_string());
    }

    match cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        Ok(mut child) => {
            // Reset server state
            {
                let mut state = server_state.lock().unwrap();
                state.is_ready = false;
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
                );
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
