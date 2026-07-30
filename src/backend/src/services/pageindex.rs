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
