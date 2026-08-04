//! Comprehensive tests for ChromaDB client
//!
//! These tests cover:
//! - Metadata conversion utilities
//! - Ollama configuration
//! - Collection operations (against the in-process `MockChroma` server)
//! - Integration tests (require ChromaDB server *and* Ollama)

/// Whole-client tests driven against [`MockChroma`], which speaks the subset of
/// the ChromaDB v2 REST API that this client uses.
///
/// Every test here holds the `CHROMA_ENDPOINT` lock while building its client,
/// because `ChromaDBClient::new` configures the underlying `chroma` client
/// through that process-global environment variable.
#[cfg(test)]
mod client_tests {
    use super::super::*;
    use crate::test_support::{
        lock_chroma_endpoint, MockChroma, MockChromaCollection, MockChromaConfig,
        UNPARSEABLE_CHROMA_ENDPOINT,
    };

    #[test]
    fn test_client_creation_accepts_a_well_formed_endpoint() {
        let _guard = lock_chroma_endpoint();
        // Building a client makes no request, so an address nothing listens on
        // is enough - and makes it obvious that no real server is involved.
        assert!(ChromaDBClient::new("http://127.0.0.1:1").is_ok());
    }

    #[test]
    fn test_client_creation_rejects_an_unparseable_endpoint() {
        let _guard = lock_chroma_endpoint();
        let error = match ChromaDBClient::new(UNPARSEABLE_CHROMA_ENDPOINT) {
            Ok(_) => panic!("An unparseable endpoint should not produce a client"),
            Err(error) => error,
        };
        assert!(error.to_string().contains(&format!(
            "Failed to create ChromaDB client with endpoint: {}",
            UNPARSEABLE_CHROMA_ENDPOINT
        )));
    }

    #[actix_web::test]
    async fn test_health_check_is_true_when_collections_can_be_listed() {
        let chroma = MockChroma::start(MockChromaConfig::empty()).await;

        let healthy = {
            let _guard = lock_chroma_endpoint();
            let client = ChromaDBClient::new(&chroma.base_url).unwrap();
            client.health_check().await.unwrap()
        };

        assert!(healthy);
        // The probe is a list request capped at 10, not a dedicated endpoint.
        let requests = chroma.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].query, "limit=10");

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_health_check_is_false_when_the_server_rejects_the_probe() {
        let chroma = MockChroma::start(MockChromaConfig {
            list_status: Some(400),
            ..MockChromaConfig::empty()
        })
        .await;

        let healthy = {
            let _guard = lock_chroma_endpoint();
            let client = ChromaDBClient::new(&chroma.base_url).unwrap();
            // A failed probe is reported as "not healthy", not as an error.
            client.health_check().await.unwrap()
        };

        assert!(!healthy);

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_health_check_is_false_when_nothing_is_listening() {
        let healthy = {
            let _guard = lock_chroma_endpoint();
            let client = ChromaDBClient::new("http://127.0.0.1:1").unwrap();
            client.health_check().await.unwrap()
        };

        assert!(!healthy);
    }

    #[actix_web::test]
    async fn test_collection_lifecycle() {
        let chroma = MockChroma::start(MockChromaConfig::holding(vec![MockChromaCollection::new(
            "existing",
        )
        .with_count(2)]))
        .await;

        let _guard = lock_chroma_endpoint();
        let client = ChromaDBClient::new(&chroma.base_url).unwrap();

        // Create
        let created = client.create_collection("notes", None).await.unwrap();
        assert_eq!(created.name, "notes");

        // Get
        let retrieved = client.get_collection("notes").await.unwrap();
        assert_eq!(retrieved.name, "notes");
        assert_eq!(retrieved.id, created.id);

        // List sees both the pre-existing collection and the new one
        let collections = client.list_collections().await.unwrap();
        let names: Vec<&str> = collections.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["existing", "notes"]);
        assert_eq!(collections[0].count, Some(2));

        // Delete
        client.delete_collection("notes").await.unwrap();

        let collections_after = client.list_collections().await.unwrap();
        assert!(!collections_after.iter().any(|c| c.name == "notes"));
        assert_eq!(chroma.collection_names(), vec!["existing".to_string()]);

        // And the deleted collection is no longer retrievable.
        assert!(client.get_collection("notes").await.is_err());

        chroma.stop().await;
    }
}

#[cfg(test)]
mod integration_tests {
    use super::super::*;
    use crate::api::chromadb::types::{AddDocumentsRequest, QueryRequest};
    use std::collections::HashMap;
    use uuid::Uuid;

    // This test requires both a running ChromaDB server and a running Ollama
    // instance (document ingestion and querying both generate embeddings), so it
    // is never run as part of the normal suite.

    #[tokio::test]
    #[ignore]
    async fn test_add_and_query_documents() {
        let client = ChromaDBClient::new("http://localhost:8000").unwrap();
        let test_collection = format!("test_query_{}", Uuid::new_v4());

        // Create collection
        client
            .create_collection(&test_collection, None)
            .await
            .unwrap();

        // Add documents
        let doc_ids: Vec<String> = (0..3).map(|_| Uuid::new_v4().to_string()).collect();
        let request = AddDocumentsRequest {
            collection: test_collection.clone(),
            ids: doc_ids.clone(),
            documents: vec![
                "This is a test document about Rust".to_string(),
                "Another document about programming".to_string(),
                "A third document about ChromaDB".to_string(),
            ],
            metadatas: Some(vec![
                {
                    let mut m = HashMap::new();
                    m.insert("source".to_string(), "test".to_string());
                    m
                },
                {
                    let mut m = HashMap::new();
                    m.insert("source".to_string(), "test".to_string());
                    m
                },
                {
                    let mut m = HashMap::new();
                    m.insert("source".to_string(), "test".to_string());
                    m
                },
            ]),
        };

        client
            .add_documents(request, "nomic-embed-text", None)
            .await
            .unwrap();

        // Query
        let query_request = QueryRequest {
            collection: test_collection.clone(),
            query_texts: vec!["Rust programming".to_string()],
            n_results: Some(2),
            where_clause: None,
        };

        let results = client
            .query(query_request, "nomic-embed-text")
            .await
            .unwrap();
        assert!(!results.ids.is_empty());
        assert_eq!(results.ids.len(), 1); // One query
        assert!(!results.ids[0].is_empty()); // Should have results

        // Cleanup
        client.delete_collection(&test_collection).await.unwrap();
    }
}

#[cfg(test)]
mod unit_tests {
    use super::super::metadata::*;
    use super::super::ollama::*;
    use chroma::types::MetadataValue;
    use std::collections::HashMap;

    #[test]
    fn test_metadata_conversion_roundtrip() {
        let mut original = HashMap::new();
        original.insert("key1".to_string(), "value1".to_string());
        original.insert("key2".to_string(), "value2".to_string());

        let chromadb_meta = to_chromadb_metadata(Some(original.clone()));
        assert!(chromadb_meta.is_some());

        let converted_back = chromadb_meta.as_ref().map(chromadb_metadata_to_hashmap);
        assert_eq!(converted_back, Some(original));
    }

    #[test]
    fn test_metadata_value_conversions() {
        // Test all MetadataValue types
        let str_val = MetadataValue::Str("test".to_string());
        assert_eq!(metadata_value_to_string(&str_val), "test");
        assert!(matches!(
            metadata_value_to_json(&str_val),
            serde_json::Value::String(_)
        ));

        let int_val = MetadataValue::Int(42);
        assert_eq!(metadata_value_to_string(&int_val), "42");
        assert!(matches!(
            metadata_value_to_json(&int_val),
            serde_json::Value::Number(_)
        ));

        let float_val = MetadataValue::Float(3.15); // Using 3.15 for testing (avoid clippy approx_constant)
        assert_eq!(metadata_value_to_string(&float_val), "3.15");
        assert!(matches!(
            metadata_value_to_json(&float_val),
            serde_json::Value::Number(_)
        ));

        let bool_val = MetadataValue::Bool(true);
        assert_eq!(metadata_value_to_string(&bool_val), "true");
        assert!(matches!(
            metadata_value_to_json(&bool_val),
            serde_json::Value::Bool(_)
        ));
    }

    #[test]
    fn test_ollama_config_customization() {
        let config = OllamaConfig {
            host: "http://custom".to_string(),
            model: "custom-model".to_string(),
            port: 9999,
            max_retries: 10,
            retry_delay_ms: 500,
        };

        assert_eq!(config.host, "http://custom");
        assert_eq!(config.model, "custom-model");
        assert_eq!(config.port, 9999);
        assert_eq!(config.max_retries, 10);
        assert_eq!(config.retry_delay_ms, 500);
    }

    #[test]
    fn test_vec_to_chromadb_metadata() {
        let mut meta1 = HashMap::new();
        meta1.insert("key1".to_string(), "value1".to_string());

        let mut meta2 = HashMap::new();
        meta2.insert("key2".to_string(), "value2".to_string());

        let result = vec_to_chromadb_metadata(vec![meta1, meta2]);
        assert_eq!(result.len(), 2);
        assert!(result[0].is_some());
        assert!(result[1].is_some());
    }
}
