use actix_web::{post, web, HttpResponse, Result as ActixResult};
use serde::Serialize;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::Mutex;

use crate::api::llama_server::logs_reader::spawn_log_reader;
use crate::api::llama_server::types::{Config, LogBuffer, ProcessHandle, ServerStateHandle};
use crate::api::llama_server::websocket::WebSocketServer;
use actix::Addr;

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
    ws_server: web::Data<Addr<WebSocketServer>>,
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
    drop(config_guard);

    // Start the llama-server process
    println!("🚀 Starting llama-server with model: {}, ctx_size: {}", hf_model, ctx_size);
    match Command::new("llama-server")
        .arg("-hf")
        .arg(&hf_model)
        .arg("--ctx-size")
        .arg(&ctx_size)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
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
                    Some(ws_server.get_ref().clone()),
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
            println!("❌ Failed to start llama server: {}", e);
            Ok(HttpResponse::InternalServerError().json(LlamaServerResponse {
                success: false,
                message: format!("Failed to start llama server: {}", e),
            }))
        }
    }
}

