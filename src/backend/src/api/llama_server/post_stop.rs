use actix_web::{post, web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};

use crate::api::llama_server::types::{ProcessHandle, ServerStateHandle};
use crate::api::llama_server::websocket::WebSocketState;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug)]
pub struct LlamaServerResponse {
    pub success: bool,
    pub message: String,
}

#[post("/api/llama-server/stop")]
pub async fn post_stop_llama_server(
    process: web::Data<ProcessHandle>,
    server_state: web::Data<ServerStateHandle>,
    ws_state: web::Data<Arc<WebSocketState>>,
) -> ActixResult<HttpResponse> {
    let mut process_guard = process.lock().unwrap();

    if let Some(ref mut child) = *process_guard {
        println!("🛑 Stopping llama-server...");
        match child.kill() {
            Ok(_) => {
                // Wait for the process to exit
                let _ = child.wait();
                *process_guard = None;

                // Reset server state
                let mut state = server_state.lock().unwrap();
                state.is_ready = false;
                drop(state);

                // Broadcast stopped status
                println!("📡 Broadcasting server stopped status");
                ws_state.broadcast_status(false, 8080);

                println!("✅ Llama server stopped successfully");
                Ok(HttpResponse::Ok().json(LlamaServerResponse {
                    success: true,
                    message: "Llama server stopped successfully".to_string(),
                }))
            }
            Err(e) => {
                println!("Failed to stop llama server: {}", e);
                Ok(
                    HttpResponse::InternalServerError().json(LlamaServerResponse {
                        success: false,
                        message: format!("Failed to stop llama server: {}", e),
                    }),
                )
            }
        }
    } else {
        Ok(HttpResponse::Ok().json(LlamaServerResponse {
            success: false,
            message: "Llama server is not running".to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::llama_server::types::{ProcessHandle, ServerState, ServerStateHandle};
    use actix_web::{test, web, App};
    use std::process::{Child, Command, Stdio};
    use std::sync::{Arc, Mutex};
    use tokio::sync::mpsc::UnboundedReceiver;

    /// A trivially cheap, long-enough-lived stand-in for a running server, so the
    /// kill path is exercised without ever launching the real `llama-server`.
    fn spawn_harmless_child() -> Child {
        Command::new("sleep")
            .arg("30")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("`sleep` must be available to run these tests")
    }

    struct Fixture {
        process: ProcessHandle,
        server_state: ServerStateHandle,
        ws_state: Arc<WebSocketState>,
        rx: UnboundedReceiver<String>,
    }

    fn fixture(is_ready: bool) -> Fixture {
        let process = ProcessHandle(Arc::new(Mutex::new(None)));
        let server_state: ServerStateHandle = Arc::new(Mutex::new(ServerState {
            is_ready,
            generation: 4,
        }));
        let log_buffer = Arc::new(Mutex::new(std::collections::VecDeque::new()));
        let ws_state = Arc::new(WebSocketState::new(
            web::Data::new(log_buffer),
            web::Data::new(process.clone()),
            web::Data::new(server_state.clone()),
        ));
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        ws_state.add_status_client("test-client".to_string(), tx);

        Fixture {
            process,
            server_state,
            ws_state,
            rx,
        }
    }

    async fn call_stop(f: &Fixture) -> (u16, LlamaServerResponse) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(f.process.clone()))
                .app_data(web::Data::new(f.server_state.clone()))
                .app_data(web::Data::new(f.ws_state.clone()))
                .service(post_stop_llama_server),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/llama-server/stop")
            .to_request();
        let resp = test::call_service(&app, req).await;
        let status = resp.status().as_u16();
        (status, test::read_body_json(resp).await)
    }

    fn drain(rx: &mut UnboundedReceiver<String>) -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            out.push(serde_json::from_str(&msg).unwrap());
        }
        out
    }

    #[actix_web::test]
    async fn test_post_stop_llama_server_kills_clears_and_broadcasts() {
        let mut f = fixture(true);
        *f.process.lock().unwrap() = Some(spawn_harmless_child());

        let (status, body) = call_stop(&f).await;

        assert_eq!(status, 200);
        assert!(body.success);
        assert_eq!(body.message, "Llama server stopped successfully");
        assert!(
            f.process.lock().unwrap().is_none(),
            "the handle must be cleared after a successful stop"
        );
        assert!(
            !f.server_state.lock().unwrap().is_ready,
            "readiness must be reset on stop"
        );

        let broadcasts = drain(&mut f.rx);
        assert_eq!(broadcasts.len(), 1);
        assert_eq!(broadcasts[0]["type"], "status");
        assert_eq!(broadcasts[0]["active"], false);
        assert_eq!(broadcasts[0]["port"], 8080);
    }

    #[actix_web::test]
    async fn test_post_stop_llama_server_reaps_an_already_exited_process() {
        let mut f = fixture(true);
        let mut child = Command::new("sleep")
            .arg("0")
            .spawn()
            .expect("`sleep` must be available to run these tests");
        let _ = child.wait();
        *f.process.lock().unwrap() = Some(child);

        let (status, body) = call_stop(&f).await;

        // `kill` on an already-reaped child succeeds, so this still reports success.
        assert_eq!(status, 200);
        assert!(body.success);
        assert!(f.process.lock().unwrap().is_none());
        assert_eq!(drain(&mut f.rx).len(), 1);
    }

    #[actix_web::test]
    async fn test_post_stop_llama_server_not_running() {
        let mut f = fixture(false);

        let (status, body) = call_stop(&f).await;

        assert_eq!(status, 200);
        assert!(!body.success);
        assert!(body.message.contains("not running"));
        assert!(
            drain(&mut f.rx).is_empty(),
            "nothing to announce when no server was running"
        );
    }
}
