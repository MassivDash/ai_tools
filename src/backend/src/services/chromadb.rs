use actix_web::web::ServiceConfig;

use crate::api::chromadb::collections::create_collection::create_collection;
use crate::api::chromadb::collections::delete_collection::delete_collection;
use crate::api::chromadb::collections::get_collection::get_collection;
use crate::api::chromadb::collections::get_collections::get_collections;
use crate::api::chromadb::documents::upload::upload_documents;
use crate::api::chromadb::health::get_chromadb_health;
use crate::api::chromadb::query::search_collection;

/// Configures all ChromaDB related endpoints
pub fn configure_chromadb_services(cfg: &mut ServiceConfig) {
    cfg.service(get_chromadb_health)
        .service(get_collections)
        .service(create_collection)
        .service(get_collection)
        .service(delete_collection)
        .service(search_collection)
        .service(upload_documents);
}

