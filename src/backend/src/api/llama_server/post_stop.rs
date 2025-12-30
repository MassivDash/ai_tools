use actix_web::{post, web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};

use crate::api::llama_server::types::{ProcessHandle, ServerStateHandle};

#[derive(Serialize, Deserialize, Debug)]
pub struct LlamaServerResponse {
    pub success: bool,
    pub message: String,
}

#[post("/api/llama-server/stop")]
pub async fn post_stop_llama_server(
    process: web::Data<ProcessHandle>,
    server_state: web::Data<ServerStateHandle>,
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
    use std::sync::{Arc, Mutex};

    #[actix_web::test]
    async fn test_post_stop_llama_server_not_running() {
        let process: ProcessHandle = Arc::new(Mutex::new(None));
        let server_state: ServerStateHandle = Arc::new(Mutex::new(ServerState { is_ready: false }));

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(process))
                .app_data(web::Data::new(server_state))
                .service(post_stop_llama_server),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/llama-server/stop")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: LlamaServerResponse = test::read_body_json(resp).await;
        assert!(!body.success);
        assert!(body.message.contains("not running"));
    }
}
