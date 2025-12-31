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

    // Determine final active status
    let active = if is_active {
        // If we are managing the process, explicitly wait for the ready signal
        // ignoring port check to avoid false positives during model download/loading
        is_ready
    } else {
        // If we aren't managing a process, do NOT report active, even if port is open.
        // This avoids false positives from stuck/zombie processes or other services.
        false
    };

    Ok(HttpResponse::Ok().json(LlamaServerStatus { active, port: 8080 }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::llama_server::types::{ProcessHandle, ServerState, ServerStateHandle};
    use actix_web::{test, web, App};
    use std::sync::{Arc, Mutex};

    #[actix_web::test]
    async fn test_get_llama_server_status_no_process() {
        let process: ProcessHandle = Arc::new(Mutex::new(None));
        let server_state: ServerStateHandle = Arc::new(Mutex::new(ServerState {
            is_ready: false,
            generation: 0,
        }));

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
        let server_state: ServerStateHandle = Arc::new(Mutex::new(ServerState {
            is_ready: true,
            generation: 0,
        }));

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
