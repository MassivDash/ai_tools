use actix_web::web::ServiceConfig;

use crate::api::model_notes::{create_or_update_model_note, delete_model_note, get_model_notes};

/// Configures all model notes related endpoints
pub fn configure_model_notes_services(cfg: &mut ServiceConfig) {
    cfg.service(get_model_notes)
        .service(create_or_update_model_note)
        .service(delete_model_note);
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn test_configure_model_notes_services_registers_all_endpoints() {
        let app = test::init_service(App::new().configure(configure_model_notes_services)).await;

        // Test that all endpoints are registered by checking they respond (even if with errors)
        let endpoints = vec![
            ("/api/model-notes", "GET"),
            ("/api/model-notes", "POST"),
            ("/api/model-notes/llama/test-model", "DELETE"),
        ];

        for (path, method) in endpoints {
            let req = match method {
                "GET" => test::TestRequest::get().uri(path),
                "POST" => test::TestRequest::post().uri(path),
                "DELETE" => test::TestRequest::delete().uri(path),
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
