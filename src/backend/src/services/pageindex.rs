use actix_web::web::ServiceConfig;

use crate::api::pageindex::documents::delete::delete_document;
use crate::api::pageindex::documents::get::get_document;
use crate::api::pageindex::documents::list::list_documents;
use crate::api::pageindex::documents::upload::upload_document;
use crate::api::pageindex::websocket::ws_handler;

/// Configures all PageIndex related endpoints
pub fn configure_pageindex_services(cfg: &mut ServiceConfig) {
    cfg.service(list_documents)
        .service(get_document)
        .service(upload_document)
        .service(delete_document)
        .route(
            "/api/pageindex/logs/ws",
            actix_web::web::get().to(ws_handler),
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    /// Registration is checked by confirming each route resolves to *something*
    /// other than a 404. The handlers all require app data that is deliberately
    /// not provided, so extraction fails with a 500 before any handler body (and
    /// therefore any database or llama-server call) runs.
    #[actix_web::test]
    async fn test_configure_pageindex_services_registers_every_route() {
        let app = test::init_service(App::new().configure(configure_pageindex_services)).await;

        let routes = [
            ("GET", "/api/pageindex/documents"),
            ("GET", "/api/pageindex/documents/some-id"),
            ("POST", "/api/pageindex/documents/upload"),
            ("DELETE", "/api/pageindex/documents/some-id"),
            ("GET", "/api/pageindex/logs/ws"),
        ];

        for (method, path) in routes {
            let req = match method {
                "GET" => test::TestRequest::get(),
                "POST" => test::TestRequest::post(),
                _ => test::TestRequest::delete(),
            }
            .uri(path)
            .to_request();

            let resp = test::call_service(&app, req).await;
            assert_ne!(
                resp.status().as_u16(),
                404,
                "{} {} should be registered",
                method,
                path
            );
        }
    }

    #[actix_web::test]
    async fn test_unregistered_pageindex_paths_are_not_found() {
        let app = test::init_service(App::new().configure(configure_pageindex_services)).await;

        let req = test::TestRequest::get()
            .uri("/api/pageindex/nope")
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status().as_u16(), 404);
    }
}
