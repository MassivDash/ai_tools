use crate::api::chromadb::config::types::{ChromaDBConfig, ConfigRequest, ConfigUpdateResponse};
use crate::api::default_configs::{ChromaDBDefaultConfig, DefaultConfigsStorage};
use actix_web::{post, web, HttpResponse, Result as ActixResult};
use std::sync::{Arc, Mutex};

#[post("/api/chromadb/config")]
pub async fn post_chromadb_config(
    request: web::Json<ConfigRequest>,
    config: web::Data<Arc<Mutex<ChromaDBConfig>>>,
    default_configs: web::Data<Arc<DefaultConfigsStorage>>,
) -> ActixResult<HttpResponse> {
    println!("💾 Updating ChromaDB configuration...");

    // Validate that embedding_model is not empty
    if request.embedding_model.trim().is_empty() {
        return Ok(HttpResponse::BadRequest().json(ConfigUpdateResponse {
            success: false,
            message: "Embedding model cannot be empty".to_string(),
        }));
    }

    let embedding_model = request.embedding_model.clone();
    let query_model = request
        .query_model
        .clone()
        .unwrap_or_else(|| embedding_model.clone());

    // Update config (drop lock before await)
    {
        let mut config_guard = config.lock().unwrap();
        config_guard.embedding_model = embedding_model.clone();
        config_guard.query_model = query_model.clone();

        if let Some(size) = request.chunk_size {
            config_guard.chunk_size = size;
        }
        if let Some(overlap) = request.chunk_overlap {
            config_guard.chunk_overlap = overlap;
        }

        println!(
            "✅ Updated config - Embedding: {}, Chunk: {}/{}",
            config_guard.embedding_model, config_guard.chunk_size, config_guard.chunk_overlap
        );
    } // Drop lock here

    // Save as default config (embedding_model is primary for chromadb) - lock is dropped
    if let Err(e) = default_configs
        .set_chromadb_default(&ChromaDBDefaultConfig {
            embedding_model: embedding_model.clone(),
        })
        .await
    {
        println!("⚠️  Failed to save chromadb default config: {}", e);
    } else {
        println!("✅ Saved chromadb default config");
    }

    Ok(HttpResponse::Ok().json(ConfigUpdateResponse {
        success: true,
        message: "Configuration updated successfully".to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    /// A throwaway default-config store on a real SQLite file.
    ///
    /// It has to be on disk: a `:memory:` database lives inside a single
    /// connection, so a pool that reconnects lands on a fresh empty database and
    /// writes appear to vanish. The returned `TempDir` must outlive the storage.
    async fn storage() -> (tempfile::TempDir, Arc<DefaultConfigsStorage>) {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let storage = DefaultConfigsStorage::new(dir.path().join("default_configs.db"))
            .await
            .expect("Failed to create test default configs storage");
        (dir, Arc::new(storage))
    }

    async fn post(
        config: Arc<Mutex<ChromaDBConfig>>,
        default_configs: Arc<DefaultConfigsStorage>,
        request: ConfigRequest,
    ) -> (u16, ConfigUpdateResponse) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .app_data(web::Data::new(default_configs))
                .service(post_chromadb_config),
        )
        .await;

        let resp = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/api/chromadb/config")
                .set_json(&request)
                .to_request(),
        )
        .await;

        let status = resp.status().as_u16();
        (status, test::read_body_json(resp).await)
    }

    #[actix_web::test]
    async fn test_post_config_updates_every_field_and_persists_the_model() {
        let config = Arc::new(Mutex::new(ChromaDBConfig::default()));
        let (_dir, default_configs) = storage().await;

        let (status, body) = post(
            Arc::clone(&config),
            Arc::clone(&default_configs),
            ConfigRequest {
                embedding_model: "mxbai-embed-large".to_string(),
                query_model: Some("snowflake-arctic-embed".to_string()),
                chunk_size: Some(1024),
                chunk_overlap: Some(128),
            },
        )
        .await;

        assert_eq!(status, 200);
        assert!(body.success);
        assert_eq!(body.message, "Configuration updated successfully");

        {
            let guard = config.lock().unwrap();
            assert_eq!(guard.embedding_model, "mxbai-embed-large");
            assert_eq!(guard.query_model, "snowflake-arctic-embed");
            assert_eq!(guard.chunk_size, 1024);
            assert_eq!(guard.chunk_overlap, 128);
        }

        // The embedding model is remembered as the ChromaDB default.
        let stored = default_configs.get_chromadb_default().await.unwrap();
        assert_eq!(stored.unwrap().embedding_model, "mxbai-embed-large");
    }

    /// An omitted `query_model` mirrors the embedding model, and omitted chunk
    /// settings are left as they were.
    #[actix_web::test]
    async fn test_post_config_defaults_the_query_model_and_keeps_the_chunking() {
        let config = Arc::new(Mutex::new(ChromaDBConfig {
            embedding_model: "old-model".to_string(),
            query_model: "old-query-model".to_string(),
            chunk_size: 777,
            chunk_overlap: 66,
        }));

        let (_dir, default_configs) = storage().await;

        let (status, body) = post(
            Arc::clone(&config),
            default_configs,
            ConfigRequest {
                embedding_model: "mxbai-embed-large".to_string(),
                query_model: None,
                chunk_size: None,
                chunk_overlap: None,
            },
        )
        .await;

        assert_eq!(status, 200);
        assert!(body.success);

        let guard = config.lock().unwrap();
        assert_eq!(guard.embedding_model, "mxbai-embed-large");
        assert_eq!(guard.query_model, "mxbai-embed-large");
        assert_eq!(guard.chunk_size, 777);
        assert_eq!(guard.chunk_overlap, 66);
    }

    #[actix_web::test]
    async fn test_post_config_rejects_a_blank_embedding_model() {
        let config = Arc::new(Mutex::new(ChromaDBConfig::default()));
        let (_dir, default_configs) = storage().await;

        let (status, body) = post(
            Arc::clone(&config),
            Arc::clone(&default_configs),
            ConfigRequest {
                embedding_model: "   ".to_string(),
                query_model: Some("something".to_string()),
                chunk_size: Some(1),
                chunk_overlap: Some(1),
            },
        )
        .await;

        assert_eq!(status, 400);
        assert!(!body.success);
        assert_eq!(body.message, "Embedding model cannot be empty");

        // Nothing was written, in memory or on disk.
        {
            let guard = config.lock().unwrap();
            assert_eq!(guard.embedding_model, "nomic-embed-text");
            assert_eq!(guard.chunk_size, 384);
        }
        assert!(default_configs
            .get_chromadb_default()
            .await
            .unwrap()
            .is_none());
    }

    /// A second update overwrites the persisted default rather than adding to it.
    #[actix_web::test]
    async fn test_post_config_overwrites_a_previously_persisted_default() {
        let config = Arc::new(Mutex::new(ChromaDBConfig::default()));
        let (_dir, default_configs) = storage().await;

        for model in ["first-model", "second-model"] {
            let (status, _) = post(
                Arc::clone(&config),
                Arc::clone(&default_configs),
                ConfigRequest {
                    embedding_model: model.to_string(),
                    query_model: None,
                    chunk_size: None,
                    chunk_overlap: None,
                },
            )
            .await;
            assert_eq!(status, 200);
        }

        let stored = default_configs.get_chromadb_default().await.unwrap();
        assert_eq!(stored.unwrap().embedding_model, "second-model");
    }
}
