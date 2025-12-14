use actix_web::{get, web, HttpResponse, Result as ActixResult};
use serde::Serialize;

use crate::api::llama_server::types::{ProcessHandle, ServerStateHandle};

#[derive(Serialize, Debug)]
pub struct LlamaServerStatus {
    pub active: bool,
    pub port: u16,
}

#[get("/api/llama-server/status")]
pub async fn get_llama_server_status(
    process: web::Data<ProcessHandle>,
    server_state: web::Data<ServerStateHandle>,
) -> ActixResult<HttpResponse> {
    // Check if process is still running
    let is_active = {
        let mut process_guard = process.lock().unwrap();
        if let Some(ref mut child) = *process_guard {
            match child.try_wait() {
                Ok(Some(_)) => {
                    // Process has exited
                    drop(process_guard);
                    let mut p = process.lock().unwrap();
                    *p = None;
                    false
                }
                Ok(None) => {
                    // Process is still running
                    true
                }
                Err(_) => {
                    // Error checking process
                    false
                }
            }
        } else {
            false
        }
    };

    // Check server state (ready message detected)
    let is_ready = {
        let state_guard = server_state.lock().unwrap();
        state_guard.is_ready
    };

    // Also check if port 8080 is listening as a fallback
    let port_check = check_port_8080().await;

    Ok(HttpResponse::Ok().json(LlamaServerStatus {
        active: is_active && (is_ready || port_check),
        port: 8080,
    }))
}

async fn check_port_8080() -> bool {
    use tokio::net::TcpStream;
    use tokio::time::{timeout, Duration};

    // Try to connect to localhost:8080 with a timeout
    matches!(
        timeout(
            Duration::from_millis(100),
            TcpStream::connect("127.0.0.1:8080")
        )
        .await,
        Ok(Ok(_))
    )
}
