use actix_web::web::ServiceConfig;

use crate::api::pdf_to_markdown::post::convert_pdf_to_markdown;
use crate::api::text_to_tokens::post::convert_text_to_tokens;
use crate::api::url_to_markdown::post::convert_url_to_markdown;

/// Configures all converter related endpoints
pub fn configure_converter_services(cfg: &mut ServiceConfig) {
    cfg.service(convert_url_to_markdown)
        .service(convert_pdf_to_markdown)
        .service(convert_text_to_tokens);
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn test_configure_converter_services_registers_all_endpoints() {
        let app = test::init_service(
            App::new().configure(configure_converter_services),
        )
        .await;

        // Test that all endpoints are registered by checking they respond (even if with errors)
        let endpoints = vec![
            ("/api/url-to-markdown", "POST"),
            ("/api/pdf-to-markdown", "POST"),
            ("/api/text-to-tokens", "POST"),
        ];

        for (path, method) in endpoints {
            let req = test::TestRequest::post().uri(path).to_request();
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
