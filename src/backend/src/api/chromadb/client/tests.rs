//! Comprehensive tests for ChromaDB client
//!
//! These tests cover:
//! - Metadata conversion utilities
//! - Ollama configuration
//! - Collection operations (with mocks where needed)
//! - Integration tests (require ChromaDB server)

#[cfg(test)]
mod integration_tests {
    use super::super::*;
    use crate::api::chromadb::types::{AddDocumentsRequest, QueryRequest};
    use std::collections::HashMap;
    use uuid::Uuid;

    // Note: These tests require a running ChromaDB server
    // They are marked as integration tests and should be run separately
    // For unit tests, see individual module test blocks

    #[tokio::test]
    #[ignore] // Ignore by default - requires ChromaDB server
    async fn test_client_creation() {
        let client = ChromaDBClient::new("http://localhost:8000");
        assert!(client.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_health_check() {
        let client = ChromaDBClient::new("http://localhost:8000").unwrap();
        let health = client.health_check().await;
        assert!(health.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_collection_lifecycle() {
        let client = ChromaDBClient::new("http://localhost:8000").unwrap();
        let test_collection = format!("test_collection_{}", Uuid::new_v4());

        // Create collection
        let created = client
            .create_collection(&test_collection, None)
            .await
            .unwrap();
        assert_eq!(created.name, test_collection);

        // Get collection
        let retrieved = client.get_collection(&test_collection).await.unwrap();
        assert_eq!(retrieved.name, test_collection);

        // List collections
        let collections = client.list_collections().await.unwrap();
        assert!(collections.iter().any(|c| c.name == test_collection));

        // Delete collection
        client.delete_collection(&test_collection).await.unwrap();

        // Verify deletion
        let collections_after = client.list_collections().await.unwrap();
        assert!(!collections_after.iter().any(|c| c.name == test_collection));
    }

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
            .add_documents(request, "nomic-embed-text")
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
