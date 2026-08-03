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
    use std::process::{Child, Command, Stdio};
    use std::sync::{Arc, Mutex};

    /// A trivially cheap, short-lived stand-in process. Never `llama-server`:
    /// the status handler only ever inspects the handle it is given.
    fn spawn_harmless_child(seconds: &str) -> Child {
        Command::new("sleep")
            .arg(seconds)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("`sleep` must be available to run these tests")
    }

    async fn call_status(
        process: &ProcessHandle,
        server_state: &ServerStateHandle,
    ) -> LlamaServerStatus {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(process.clone()))
                .app_data(web::Data::new(server_state.clone()))
                .service(get_llama_server_status),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/llama-server/status")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        test::read_body_json(resp).await
    }

    #[actix_web::test]
    async fn test_get_llama_server_status_reports_active_for_a_live_ready_process() {
        let process = ProcessHandle(Arc::new(Mutex::new(Some(spawn_harmless_child("5")))));
        let server_state: ServerStateHandle = Arc::new(Mutex::new(ServerState {
            is_ready: true,
            generation: 3,
        }));

        let body = call_status(&process, &server_state).await;

        assert!(body.active);
        assert_eq!(body.port, 8080);
        assert!(
            process.lock().unwrap().is_some(),
            "a live process must stay registered"
        );

        if let Some(child) = process.lock().unwrap().as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        };
    }

    #[actix_web::test]
    async fn test_get_llama_server_status_is_inactive_while_a_live_process_is_not_ready_yet() {
        let process = ProcessHandle(Arc::new(Mutex::new(Some(spawn_harmless_child("5")))));
        let server_state: ServerStateHandle = Arc::new(Mutex::new(ServerState {
            is_ready: false,
            generation: 1,
        }));

        let body = call_status(&process, &server_state).await;

        assert!(
            !body.active,
            "a loading/downloading server must not be reported active"
        );

        if let Some(child) = process.lock().unwrap().as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        };
    }

    #[actix_web::test]
    async fn test_get_llama_server_status_clears_an_exited_process() {
        let mut child = spawn_harmless_child("0");
        let _ = child.wait();
        let process = ProcessHandle(Arc::new(Mutex::new(Some(child))));
        // Stale readiness from the run that just died must not leak through.
        let server_state: ServerStateHandle = Arc::new(Mutex::new(ServerState {
            is_ready: true,
            generation: 2,
        }));

        let body = call_status(&process, &server_state).await;

        assert!(!body.active);
        assert!(
            process.lock().unwrap().is_none(),
            "an exited process should be cleared from the handle"
        );
    }

    #[actix_web::test]
    async fn test_get_llama_server_status_no_process() {
        let process = ProcessHandle(Arc::new(Mutex::new(None)));
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
    async fn test_get_llama_server_status_ignores_readiness_without_a_managed_process() {
        let process = ProcessHandle(Arc::new(Mutex::new(None)));
        let server_state: ServerStateHandle = Arc::new(Mutex::new(ServerState {
            is_ready: true,
            generation: 5,
        }));

        let body = call_status(&process, &server_state).await;

        assert!(
            !body.active,
            "readiness alone must not report active when no process is managed"
        );
    }

    #[actix_web::test]
    async fn test_get_llama_server_status_with_ready_state() {
        let process = ProcessHandle(Arc::new(Mutex::new(None)));
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
