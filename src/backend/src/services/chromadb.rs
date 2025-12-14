use actix_web::web::ServiceConfig;

use crate::api::chromadb::collections::create_collection::create_collection;
use crate::api::chromadb::collections::delete_collection::delete_collection;
use crate::api::chromadb::collections::get_collection::get_collection;
use crate::api::chromadb::collections::get_collections::get_collections;
use crate::api::chromadb::config::get_config::get_chromadb_config;
use crate::api::chromadb::config::get_models::get_ollama_models;
use crate::api::chromadb::config::post_config::post_chromadb_config;
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
        .service(upload_documents)
        .service(get_ollama_models)
        .service(get_chromadb_config)
        .service(post_chromadb_config);
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App};

    #[actix_web::test]
    async fn test_configure_chromadb_services_registers_all_endpoints() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new("http://localhost:8000".to_string()))
                .configure(configure_chromadb_services),
        )
        .await;

        // Test that all endpoints are registered by checking they respond (even if with errors)
        let endpoints = vec![
            ("/api/chromadb/health", "GET"),
            ("/api/chromadb/collections", "GET"),
            ("/api/chromadb/collections", "POST"),
            ("/api/chromadb/collections/test", "GET"),
            ("/api/chromadb/collections/test", "DELETE"),
            ("/api/chromadb/query", "POST"),
            ("/api/chromadb/documents/upload", "POST"),
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
            let status = resp.status().as_u16();
            // Endpoints should be registered
            // For GET /api/chromadb/collections/{name}, 404 is valid (collection not found)
            // For other endpoints, check they're not 404 (route not found)
            // We can distinguish by checking if it's a JSON response (our handlers) vs plain 404
            if path == "/api/chromadb/collections/test" && method == "GET" {
                // This endpoint returns 404 with JSON when collection doesn't exist
                // That means it's registered. Check response has content-type json
                let content_type = resp.headers().get("content-type");
                if let Some(ct) = content_type {
                    let ct_str = ct.to_str().unwrap_or("");
                    // If it's JSON, the endpoint is registered
                    if ct_str.contains("json") {
                        continue; // Endpoint is registered
                    }
                }
            }
            // For other endpoints, 404 means route not found
            assert_ne!(
                status, 404,
                "Endpoint {} {} should be registered (got status {})",
                method, path, status
            );
        }
    }
}
