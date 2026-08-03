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

    let query_request = req.into_inner();

    // Fetch the collection to see if it has an embedding_model attached to its metadata
    let mut query_model_opt = None;
    if let Ok(collection) = client.get_collection(&query_request.collection).await {
        if let Some(metadata) = collection.metadata {
            if let Some(model_value) = metadata.get("embedding_model") {
                let model_str = model_value.as_str();
                println!(
                    "📋 Found embedding_model in collection metadata: {}",
                    model_str
                );
                query_model_opt = Some(model_str.to_string());
            }
        }
    }

    let query_model = match query_model_opt {
        Some(m) => m,
        None => {
            let config_guard = chromadb_config.lock().unwrap();

            let model = config_guard.embedding_model.clone();

            if model.trim().is_empty() {
                return Ok(HttpResponse::BadRequest().json(ChromaDBResponse::<QueryResponse> {
                    success: false,
                    data: None,
                    error: Some("Embedding model is not configured. Please configure it in ChromaDB settings.".to_string()),
                    message: None,
                }));
            }
            model
        }
    };

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
    use crate::test_support::{
        lock_chroma_endpoint, MockChroma, MockChromaCollection, MockChromaConfig,
        UNPARSEABLE_CHROMA_ENDPOINT,
    };
    use actix_web::{test, web, App};

    // Note on scope: once this handler has resolved a query model it calls
    // `client.query`, which generates embeddings by spawning the real `ollama`
    // binary. The tests below therefore only drive the paths that return before
    // that point - client construction, collection lookup, model resolution and
    // request validation - and never let a query reach the embedding step.

    async fn search(
        address: &str,
        config: ChromaDBConfig,
        request: QueryRequest,
    ) -> (u16, ChromaDBResponse<QueryResponse>) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(address.to_string()))
                .app_data(web::Data::new(Arc::new(Mutex::new(config))))
                .service(search_collection),
        )
        .await;

        let resp = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/api/chromadb/query")
                .set_json(&request)
                .to_request(),
        )
        .await;

        let status = resp.status().as_u16();
        (status, test::read_body_json(resp).await)
    }

    fn query_for(collection: &str, query_texts: Vec<&str>) -> QueryRequest {
        QueryRequest {
            collection: collection.to_string(),
            query_texts: query_texts.into_iter().map(str::to_string).collect(),
            n_results: Some(5),
            where_clause: None,
        }
    }

    #[actix_web::test]
    async fn test_search_rejects_an_empty_query_text_list() {
        let chroma = MockChroma::start(MockChromaConfig::holding(vec![MockChromaCollection::new(
            "notes",
        )]))
        .await;

        let guard = lock_chroma_endpoint();
        let (status, body) = search(
            &chroma.base_url,
            ChromaDBConfig::default(),
            query_for("notes", vec![]),
        )
        .await;
        drop(guard);

        assert_eq!(status, 400);
        assert!(!body.success);
        assert_eq!(body.error.unwrap(), "Query texts cannot be empty");
        // The collection was looked up before the request was validated.
        assert!(chroma.requests()[0].path.ends_with("/collections/notes"));

        chroma.stop().await;
    }

    /// When the collection carries an `embedding_model`, that wins over the
    /// configured one - proven here by the model in config being blank yet the
    /// request getting as far as the query-text check.
    #[actix_web::test]
    async fn test_search_prefers_the_model_recorded_on_the_collection() {
        let chroma = MockChroma::start(MockChromaConfig::holding(vec![MockChromaCollection::new(
            "notes",
        )
        .with_metadata(&[("embedding_model", "mxbai-embed-large")])]))
        .await;

        let guard = lock_chroma_endpoint();
        let (status, body) = search(
            &chroma.base_url,
            ChromaDBConfig {
                embedding_model: String::new(),
                ..ChromaDBConfig::default()
            },
            query_for("notes", vec![]),
        )
        .await;
        drop(guard);

        assert_eq!(status, 400);
        assert_eq!(body.error.unwrap(), "Query texts cannot be empty");

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_search_rejects_a_blank_configured_model_when_the_collection_has_none() {
        let chroma = MockChroma::start(MockChromaConfig::holding(vec![MockChromaCollection::new(
            "notes",
        )]))
        .await;

        let guard = lock_chroma_endpoint();
        let (status, body) = search(
            &chroma.base_url,
            ChromaDBConfig {
                embedding_model: "   ".to_string(),
                ..ChromaDBConfig::default()
            },
            query_for("notes", vec!["some text"]),
        )
        .await;
        drop(guard);

        assert_eq!(status, 400);
        assert!(!body.success);
        assert_eq!(
            body.error.unwrap(),
            "Embedding model is not configured. Please configure it in ChromaDB settings."
        );

        chroma.stop().await;
    }

    /// A collection that cannot be fetched falls through to the configured model
    /// rather than failing outright.
    #[actix_web::test]
    async fn test_search_falls_back_to_the_configured_model_for_an_unknown_collection() {
        let chroma = MockChroma::start(MockChromaConfig::empty()).await;

        let guard = lock_chroma_endpoint();
        let (status, body) = search(
            &chroma.base_url,
            ChromaDBConfig {
                embedding_model: String::new(),
                ..ChromaDBConfig::default()
            },
            query_for("missing", vec!["some text"]),
        )
        .await;
        drop(guard);

        // The lookup failed silently, so the blank configured model is what gets
        // reported - not "collection not found".
        assert_eq!(status, 400);
        assert!(body
            .error
            .unwrap()
            .contains("Embedding model is not configured"));

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_search_reports_an_unusable_address_as_503() {
        let _guard = lock_chroma_endpoint();
        let (status, body) = search(
            UNPARSEABLE_CHROMA_ENDPOINT,
            ChromaDBConfig::default(),
            query_for("notes", vec!["some text"]),
        )
        .await;

        assert_eq!(status, 503);
        assert!(!body.success);
        assert!(body.data.is_none());
        assert!(body
            .error
            .unwrap()
            .contains("Failed to create ChromaDB client"));
    }
}
