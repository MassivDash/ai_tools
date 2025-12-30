use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::config::types::ChromaDBConfig;
use crate::api::chromadb::types::{ChromaDBResponse, QueryRequest, QueryResponse};
use actix_web::{post, web, HttpResponse, Result as ActixResult};
use std::sync::{Arc, Mutex};

#[post("/api/chromadb/query")]
pub async fn search_collection(
    req: web::Json<QueryRequest>,
    chroma_address: web::Data<String>,
    chromadb_config: web::Data<Arc<Mutex<ChromaDBConfig>>>,
) -> ActixResult<HttpResponse> {
    let client = match ChromaDBClient::new(chroma_address.as_str()) {
        Ok(c) => c,
        Err(e) => {
            println!("Failed to create ChromaDB client: {}", e);
            return Ok(HttpResponse::ServiceUnavailable().json(
                ChromaDBResponse::<QueryResponse> {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                    message: None,
                },
            ));
        }
    };

    // Get embedding model from config (use same model as uploads to ensure dimension match)
    // Always use embedding_model for queries to prevent dimension mismatches
    let query_model = {
        let config_guard = chromadb_config.lock().unwrap();

        // Log current config state for debugging
        println!(
            "📋 Current config - Embedding: '{}', Query: '{}'",
            config_guard.embedding_model, config_guard.query_model
        );

        // ALWAYS use embedding_model for queries to ensure dimension consistency
        // This prevents the common issue where query_model is set to a different model
        let model = config_guard.embedding_model.clone();

        if model.trim().is_empty() {
            return Ok(HttpResponse::BadRequest().json(ChromaDBResponse::<QueryResponse> {
                success: false,
                data: None,
                error: Some("Embedding model is not configured. Please configure it in ChromaDB settings.".to_string()),
                message: None,
            }));
        }

        // Warn if query_model is set but different (user might expect it to be used)
        if !config_guard.query_model.trim().is_empty()
            && config_guard.query_model != config_guard.embedding_model
        {
            println!(
                "⚠️  WARNING: Query model '{}' is set but will be ignored. Using embedding model '{}' instead to ensure dimension consistency.",
                config_guard.query_model, model
            );
        }

        println!(
            "✅ Query will use embedding model '{}' (must match upload model)",
            model
        );
        println!("🔍 IMPORTANT: If you get dimension mismatch errors, verify that:");
        println!("   1. Your config has embedding_model set to: '{}'", model);
        println!("   2. You uploaded documents using this same model");
        println!("   3. The model name matches exactly (including :latest tag if present)");
        model
    };

    let query_request = req.into_inner();

    // Validate query request
    if query_request.query_texts.is_empty() {
        return Ok(
            HttpResponse::BadRequest().json(ChromaDBResponse::<QueryResponse> {
                success: false,
                data: None,
                error: Some("Query texts cannot be empty".to_string()),
                message: None,
            }),
        );
    }

    println!(
        "🔍 Query will use model '{}' (must match the model used for document uploads)",
        query_model
    );

    match client.query(query_request, &query_model).await {
        Ok(results) => Ok(HttpResponse::Ok().json(ChromaDBResponse {
            success: true,
            data: Some(results),
            error: None,
            message: None,
        })),
        Err(e) => {
            // Get the root error message without duplication
            let error_msg = e.to_string();
            println!("Query failed: {}", error_msg);

            // If the error message already contains detailed information, use it directly
            // Otherwise, try to get more context from the error chain
            let detailed_error = if error_msg.contains('\n') || error_msg.len() > 100 {
                // Error already has detailed information
                error_msg
            } else {
                // Try to get more context from error chain
                let mut full_error = error_msg.clone();
                for (i, cause) in e.chain().skip(1).enumerate() {
                    if i == 0 {
                        full_error.push_str(&format!(": {}", cause));
                    }
                }
                full_error
            };

            Ok(
                HttpResponse::InternalServerError().json(ChromaDBResponse::<QueryResponse> {
                    success: false,
                    data: None,
                    error: Some(detailed_error),
                    message: None,
                }),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::chromadb::types::QueryRequest;
    use actix_web::{test, web, App};

    #[actix_web::test]
    async fn test_search_collection_basic() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new("http://localhost:8000".to_string()))
                .service(search_collection),
        )
        .await;

        let query_request = QueryRequest {
            collection: "test_collection".to_string(),
            query_texts: vec!["test query".to_string()],
            n_results: Some(5),
            where_clause: None,
        };

        let req = test::TestRequest::post()
            .uri("/api/chromadb/query")
            .set_json(&query_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        // Should not be 400 (bad request) - might be 500 if ChromaDB unavailable
        assert_ne!(resp.status().as_u16(), 400);
    }

    #[actix_web::test]
    async fn test_search_collection_empty_query() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new("http://localhost:8000".to_string()))
                .service(search_collection),
        )
        .await;

        let query_request = QueryRequest {
            collection: "test_collection".to_string(),
            query_texts: vec![],
            n_results: Some(5),
            where_clause: None,
        };

        let req = test::TestRequest::post()
            .uri("/api/chromadb/query")
            .set_json(&query_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        // Should not be 400 (bad request) - might be 500 if ChromaDB unavailable
        assert_ne!(resp.status().as_u16(), 400);
    }
}
