use actix_web::{get, web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};

use crate::api::llama_server::types::{ProcessHandle, ServerStateHandle};

#[derive(Serialize, Deserialize, Debug)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App};
    use crate::api::llama_server::types::{ProcessHandle, ServerState, ServerStateHandle};
    use std::sync::{Arc, Mutex};

    #[actix_web::test]
    async fn test_get_llama_server_status_no_process() {
        let process: ProcessHandle = Arc::new(Mutex::new(None));
        let server_state: ServerStateHandle =
            Arc::new(Mutex::new(ServerState { is_ready: false }));

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(process))
                .app_data(web::Data::new(server_state))
                .service(get_llama_server_status),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/llama-server/status")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());
        let body: LlamaServerStatus = test::read_body_json(resp).await;
        assert!(!body.active);
        assert_eq!(body.port, 8080);
    }

    #[actix_web::test]
    async fn test_get_llama_server_status_with_ready_state() {
        let process: ProcessHandle = Arc::new(Mutex::new(None));
        let server_state: ServerStateHandle =
            Arc::new(Mutex::new(ServerState { is_ready: true }));

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(process))
                .app_data(web::Data::new(server_state))
                .service(get_llama_server_status),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/llama-server/status")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());
        let body: LlamaServerStatus = test::read_body_json(resp).await;
        assert_eq!(body.port, 8080);
    }
}
