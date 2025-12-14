use actix_web::web::ServiceConfig;

use crate::api::llama_server::get_config::get_llama_config;
use crate::api::llama_server::get_logs::get_llama_logs;
use crate::api::llama_server::get_models::get_llama_models;
use crate::api::llama_server::get_status::get_llama_server_status;
use crate::api::llama_server::post_config::post_update_config;
use crate::api::llama_server::post_start::post_start_llama_server;
use crate::api::llama_server::post_stop::post_stop_llama_server;

/// Configures all llama-server related endpoints
pub fn configure_llama_server_services(cfg: &mut ServiceConfig) {
    cfg.service(get_llama_server_status)
        .service(get_llama_models)
        .service(get_llama_config)
        .service(get_llama_logs)
        .service(post_start_llama_server)
        .service(post_stop_llama_server)
        .service(post_update_config);
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn test_configure_llama_server_services_registers_all_endpoints() {
        let app = test::init_service(
            App::new().configure(configure_llama_server_services),
        )
        .await;

        // Test that all endpoints are registered by checking they respond (even if with errors)
        let endpoints = vec![
            ("/api/llama-server/status", "GET"),
            ("/api/llama-server/models", "GET"),
            ("/api/llama-server/config", "GET"),
            ("/api/llama-server/logs", "GET"),
            ("/api/llama-server/start", "POST"),
            ("/api/llama-server/stop", "POST"),
            ("/api/llama-server/config", "POST"),
        ];

        for (path, method) in endpoints {
            let req = match method {
                "GET" => test::TestRequest::get().uri(path),
                "POST" => test::TestRequest::post().uri(path),
                _ => continue,
            };
            let req = req.to_request();
            let resp = test::call_service(&app, req).await;
            // Endpoints should be registered (not 404)
            assert_ne!(
                resp.status().as_u16(),
                404,
                "Endpoint {} {} should be registered",
                method,
                path
            );
        }
    }
}
