use crate::api::chromadb::config::types::{ChromaDBConfig, ConfigResponse};
use actix_web::{get, web, HttpResponse, Result as ActixResult};
use std::sync::{Arc, Mutex};

#[get("/api/chromadb/config")]
pub async fn get_chromadb_config(
    config: web::Data<Arc<Mutex<ChromaDBConfig>>>,
) -> ActixResult<HttpResponse> {
    println!("📋 Fetching ChromaDB configuration...");

    let config_guard = config.lock().unwrap();
    let config_response = ConfigResponse {
        embedding_model: config_guard.embedding_model.clone(),
        query_model: config_guard.query_model.clone(),
        chunk_size: config_guard.chunk_size,
        chunk_overlap: config_guard.chunk_overlap,
    };

    println!(
        "✅ Current config - Embedding: {}, Query: {}",
        config_response.embedding_model, config_response.query_model
    );

    Ok(HttpResponse::Ok().json(config_response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    async fn get_config(config: Arc<Mutex<ChromaDBConfig>>) -> ConfigResponse {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .service(get_chromadb_config),
        )
        .await;

        let resp = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/api/chromadb/config")
                .to_request(),
        )
        .await;

        assert_eq!(resp.status().as_u16(), 200);
        test::read_body_json(resp).await
    }

    #[actix_web::test]
    async fn test_get_config_returns_the_defaults() {
        let body = get_config(Arc::new(Mutex::new(ChromaDBConfig::default()))).await;

        assert_eq!(body.embedding_model, "nomic-embed-text");
        assert_eq!(body.query_model, "nomic-embed-text");
        assert_eq!(body.chunk_size, 384);
        assert_eq!(body.chunk_overlap, 50);
    }

    #[actix_web::test]
    async fn test_get_config_reflects_the_live_shared_config() {
        let config = Arc::new(Mutex::new(ChromaDBConfig {
            embedding_model: "mxbai-embed-large".to_string(),
            query_model: "snowflake-arctic-embed".to_string(),
            chunk_size: 1024,
            chunk_overlap: 128,
        }));

        let body = get_config(Arc::clone(&config)).await;

        assert_eq!(body.embedding_model, "mxbai-embed-large");
        assert_eq!(body.query_model, "snowflake-arctic-embed");
        assert_eq!(body.chunk_size, 1024);
        assert_eq!(body.chunk_overlap, 128);

        // Reading did not disturb the shared config.
        let guard = config.lock().unwrap();
        assert_eq!(guard.embedding_model, "mxbai-embed-large");
    }
}
