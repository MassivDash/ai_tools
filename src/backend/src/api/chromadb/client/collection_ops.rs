//! Collection operations
//!
//! This module handles all collection-related operations: list, create, get, delete.

use crate::api::chromadb::types::Collection;
use anyhow::{Context, Result};
use chroma::ChromaHttpClient;
use std::collections::HashMap;

use super::metadata::{chromadb_metadata_to_hashmap, to_chromadb_metadata};

/// List all collections in ChromaDB
pub async fn list_collections(client: &ChromaHttpClient) -> Result<Vec<Collection>> {
    let collections = client
        .list_collections(100, None) // limit: 100, offset: None
        .await
        .context("Failed to list collections")?;

    let mut result = Vec::new();
    for collection in collections {
        // Get count properly
        let actual_count = if let Ok(col) = client.get_collection(collection.name()).await {
            col.count().await.unwrap_or(0) as usize
        } else {
            0
        };

        result.push(Collection {
            id: collection.id().to_string(),
            name: collection.name().to_string(),
            metadata: collection
                .metadata()
                .as_ref()
                .map(chromadb_metadata_to_hashmap),
            count: Some(actual_count),
        });
    }

    Ok(result)
}

/// Create a new collection
pub async fn create_collection(
    client: &ChromaHttpClient,
    name: &str,
    metadata: Option<HashMap<String, String>>,
) -> Result<Collection> {
    println!(
        "🔧 ChromaDBClient::create_collection called with name: '{}', metadata: {:?}",
        name, metadata
    );

    let metadata_map = to_chromadb_metadata(metadata);

    // Log distance metric if set
    if let Some(ref meta) = metadata_map {
        if let Some(space) = meta.get("hnsw:space") {
            println!(
                "📊 Distance metric configured: {} (via hnsw:space)",
                match space {
                    chroma::types::MetadataValue::Str(s) => s.as_str(),
                    _ => "unknown",
                }
            );
        }
    }

    println!(
        "🔧 Calling chroma client.create_collection with name: '{}', metadata_map: {:?}",
        name, metadata_map
    );

    let collection = client
        .create_collection(name, None, metadata_map) // name, schema: None, metadata
        .await
        .with_context(|| {
            format!(
                "Failed to create collection '{}'. Check if collection already exists or if ChromaDB server is accessible.",
                name
            )
        })?;

    println!(
        "✅ ChromaDB collection created successfully: {}",
        collection.name()
    );

    // Verify the collection's metadata to check if distance metric was set
    if let Some(collection_metadata) = collection.metadata() {
        println!(
            "📋 Collection metadata after creation: {:?}",
            collection_metadata
        );
        if let Some(space) = collection_metadata.get("hnsw:space") {
            println!(
                "✅ Distance metric confirmed in collection: {}",
                match space {
                    chroma::types::MetadataValue::Str(s) => s.as_str(),
                    _ => "unknown",
                }
            );
        } else {
            println!("⚠️ WARNING: Distance metric (hnsw:space) not found in collection metadata!");
            println!("   The collection may be using default L2 distance instead of cosine.");
        }
    }

    Ok(Collection {
        id: collection.id().to_string(),
        name: collection.name().to_string(),
        metadata: collection
            .metadata()
            .as_ref()
            .map(chromadb_metadata_to_hashmap),
        count: Some(0),
    })
}

/// Get a collection by name
pub async fn get_collection(client: &ChromaHttpClient, name: &str) -> Result<Collection> {
    let collection = client
        .get_collection(name)
        .await
        .context("Failed to get collection")?;

    let count = collection.count().await.unwrap_or(0) as usize;

    Ok(Collection {
        id: collection.id().to_string(),
        name: collection.name().to_string(),
        metadata: collection
            .metadata()
            .as_ref()
            .map(chromadb_metadata_to_hashmap),
        count: Some(count),
    })
}

/// Delete a collection by name
pub async fn delete_collection(client: &ChromaHttpClient, name: &str) -> Result<()> {
    client
        .delete_collection(name)
        .await
        .context("Failed to delete collection")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::api::chromadb::client::ChromaDBClient;
    use crate::test_support::{
        lock_chroma_endpoint, MockChroma, MockChromaCollection, MockChromaConfig,
    };

    /// `list_collections` fans out: one paged list request, then a lookup plus a
    /// record count per collection returned.
    #[actix_web::test]
    async fn test_list_collections_returns_every_collection_with_its_live_count() {
        let chroma = MockChroma::start(MockChromaConfig::holding(vec![
            MockChromaCollection::new("notes")
                .with_metadata(&[("embedding_model", "nomic-embed-text")])
                .with_count(7),
            MockChromaCollection::new("papers").with_count(0),
        ]))
        .await;

        let collections = {
            let _guard = lock_chroma_endpoint();
            let client = ChromaDBClient::new(&chroma.base_url).unwrap();
            client.list_collections().await.unwrap()
        };

        assert_eq!(collections.len(), 2);
        assert_eq!(collections[0].name, "notes");
        assert_eq!(collections[0].count, Some(7));
        assert_eq!(
            collections[0].metadata.as_ref().unwrap()["embedding_model"],
            "nomic-embed-text"
        );
        assert!(!collections[0].id.is_empty());
        assert_eq!(collections[1].name, "papers");
        assert_eq!(collections[1].count, Some(0));
        assert_eq!(collections[1].metadata, None);

        // The list request asks for a page of 100 and no offset, and each
        // collection is then re-fetched by name and counted by id.
        let requests = chroma.requests();
        assert_eq!(requests[0].method, "GET");
        assert_eq!(
            requests[0].path,
            "/api/v2/tenants/default_tenant/databases/default_database/collections"
        );
        assert_eq!(requests[0].query, "limit=100");
        let paths: Vec<&str> = requests.iter().map(|r| r.path.as_str()).collect();
        assert!(paths
            .iter()
            .any(|path| path.ends_with("/collections/notes")));
        assert!(paths
            .iter()
            .any(|path| path.ends_with("/collections/papers")));
        assert_eq!(
            paths.iter().filter(|path| path.ends_with("/count")).count(),
            2
        );

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_list_collections_on_an_empty_server_makes_one_request() {
        let chroma = MockChroma::start(MockChromaConfig::empty()).await;

        let collections = {
            let _guard = lock_chroma_endpoint();
            let client = ChromaDBClient::new(&chroma.base_url).unwrap();
            client.list_collections().await.unwrap()
        };

        assert!(collections.is_empty());
        assert_eq!(chroma.requests().len(), 1);

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_list_collections_propagates_a_server_error() {
        let chroma = MockChroma::start(MockChromaConfig {
            list_status: Some(400),
            ..MockChromaConfig::empty()
        })
        .await;

        let error = {
            let _guard = lock_chroma_endpoint();
            let client = ChromaDBClient::new(&chroma.base_url).unwrap();
            client.list_collections().await.unwrap_err()
        };

        assert!(error.to_string().contains("Failed to list collections"));
        // The server's own error body survives into the error chain.
        let chain = format!("{:#}", error);
        assert!(
            chain.contains("could not list collections"),
            "expected the ChromaDB error body in the chain, got: {chain}"
        );

        chroma.stop().await;
    }

    /// A collection whose record count cannot be read is still listed, with the
    /// count silently reported as zero.
    #[actix_web::test]
    async fn test_list_collections_reports_zero_when_counting_fails() {
        let chroma = MockChroma::start(MockChromaConfig {
            count_status: Some(400),
            ..MockChromaConfig::holding(vec![MockChromaCollection::new("notes").with_count(42)])
        })
        .await;

        let collections = {
            let _guard = lock_chroma_endpoint();
            let client = ChromaDBClient::new(&chroma.base_url).unwrap();
            client.list_collections().await.unwrap()
        };

        assert_eq!(collections.len(), 1);
        assert_eq!(collections[0].count, Some(0));

        chroma.stop().await;
    }

    /// A collection that vanishes between the list and the per-collection lookup
    /// is still listed, with a zero count.
    #[actix_web::test]
    async fn test_list_collections_reports_zero_when_the_lookup_fails() {
        let chroma = MockChroma::start(MockChromaConfig {
            get_status: Some(404),
            ..MockChromaConfig::holding(vec![MockChromaCollection::new("notes").with_count(42)])
        })
        .await;

        let collections = {
            let _guard = lock_chroma_endpoint();
            let client = ChromaDBClient::new(&chroma.base_url).unwrap();
            client.list_collections().await.unwrap()
        };

        assert_eq!(collections.len(), 1);
        assert_eq!(collections[0].name, "notes");
        assert_eq!(collections[0].count, Some(0));
        // No count request was ever made, because the lookup failed first.
        assert!(!chroma
            .requests()
            .iter()
            .any(|request| request.path.ends_with("/count")));

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_create_collection_posts_the_name_and_metadata_and_echoes_the_result() {
        let chroma = MockChroma::start(MockChromaConfig::empty()).await;

        let metadata = std::collections::HashMap::from([
            ("hnsw:space".to_string(), "cosine".to_string()),
            (
                "embedding_model".to_string(),
                "nomic-embed-text".to_string(),
            ),
        ]);

        let created = {
            let _guard = lock_chroma_endpoint();
            let client = ChromaDBClient::new(&chroma.base_url).unwrap();
            client
                .create_collection("notes", Some(metadata))
                .await
                .unwrap()
        };

        assert_eq!(created.name, "notes");
        // A freshly created collection is reported as empty without a round trip.
        assert_eq!(created.count, Some(0));
        let echoed = created.metadata.as_ref().unwrap();
        assert_eq!(echoed["hnsw:space"], "cosine");
        assert_eq!(echoed["embedding_model"], "nomic-embed-text");

        let requests = chroma.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "POST");
        assert_eq!(
            requests[0].path,
            "/api/v2/tenants/default_tenant/databases/default_database/collections"
        );
        let body = requests[0].body.as_ref().unwrap();
        assert_eq!(body["name"], "notes");
        assert_eq!(body["metadata"]["hnsw:space"], "cosine");
        assert_eq!(body["metadata"]["embedding_model"], "nomic-embed-text");
        assert_eq!(body["get_or_create"], false);
        assert!(body["schema"].is_null());

        // The collection really exists on the server afterwards.
        assert_eq!(chroma.collection_names(), vec!["notes".to_string()]);

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_create_collection_without_metadata_sends_a_null_metadata_field() {
        let chroma = MockChroma::start(MockChromaConfig::empty()).await;

        let created = {
            let _guard = lock_chroma_endpoint();
            let client = ChromaDBClient::new(&chroma.base_url).unwrap();
            client.create_collection("notes", None).await.unwrap()
        };

        assert_eq!(created.name, "notes");
        assert_eq!(created.metadata, None);
        assert!(chroma.requests()[0].body.as_ref().unwrap()["metadata"].is_null());

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_create_collection_reports_a_name_clash() {
        let chroma = MockChroma::start(MockChromaConfig::holding(vec![MockChromaCollection::new(
            "notes",
        )]))
        .await;

        let error = {
            let _guard = lock_chroma_endpoint();
            let client = ChromaDBClient::new(&chroma.base_url).unwrap();
            client.create_collection("notes", None).await.unwrap_err()
        };

        assert!(error
            .to_string()
            .contains("Failed to create collection 'notes'"));
        assert!(format!("{:#}", error).contains("Collection notes already exists"));
        // The clash left the server untouched.
        assert_eq!(chroma.collection_names(), vec!["notes".to_string()]);

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_get_collection_returns_the_collection_with_its_count() {
        let chroma = MockChroma::start(MockChromaConfig::holding(vec![MockChromaCollection::new(
            "notes",
        )
        .with_metadata(&[("owner", "alice")])
        .with_count(3)]))
        .await;

        let collection = {
            let _guard = lock_chroma_endpoint();
            let client = ChromaDBClient::new(&chroma.base_url).unwrap();
            client.get_collection("notes").await.unwrap()
        };

        assert_eq!(collection.name, "notes");
        assert_eq!(collection.count, Some(3));
        assert_eq!(collection.metadata.as_ref().unwrap()["owner"], "alice");

        let requests = chroma.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            requests[0].path,
            "/api/v2/tenants/default_tenant/databases/default_database/collections/notes"
        );
        // The count endpoint is addressed by id, not by name.
        assert!(requests[1]
            .path
            .ends_with(&format!("/collections/{}/count", collection.id)));

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_get_collection_reports_an_unknown_name() {
        let chroma = MockChroma::start(MockChromaConfig::empty()).await;

        let error = {
            let _guard = lock_chroma_endpoint();
            let client = ChromaDBClient::new(&chroma.base_url).unwrap();
            client.get_collection("nope").await.unwrap_err()
        };

        assert!(error.to_string().contains("Failed to get collection"));
        assert!(format!("{:#}", error).contains("Collection nope does not exist"));

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_get_collection_reports_zero_when_counting_fails() {
        let chroma = MockChroma::start(MockChromaConfig {
            count_status: Some(400),
            ..MockChromaConfig::holding(vec![MockChromaCollection::new("notes").with_count(9)])
        })
        .await;

        let collection = {
            let _guard = lock_chroma_endpoint();
            let client = ChromaDBClient::new(&chroma.base_url).unwrap();
            client.get_collection("notes").await.unwrap()
        };

        assert_eq!(collection.count, Some(0));

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_delete_collection_removes_it_from_the_server() {
        let chroma = MockChroma::start(MockChromaConfig::holding(vec![
            MockChromaCollection::new("notes"),
            MockChromaCollection::new("papers"),
        ]))
        .await;

        {
            let _guard = lock_chroma_endpoint();
            let client = ChromaDBClient::new(&chroma.base_url).unwrap();
            client.delete_collection("notes").await.unwrap();
        }

        assert_eq!(chroma.collection_names(), vec!["papers".to_string()]);
        let requests = chroma.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "DELETE");
        assert_eq!(
            requests[0].path,
            "/api/v2/tenants/default_tenant/databases/default_database/collections/notes"
        );

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_delete_collection_reports_an_unknown_name() {
        let chroma = MockChroma::start(MockChromaConfig::empty()).await;

        let error = {
            let _guard = lock_chroma_endpoint();
            let client = ChromaDBClient::new(&chroma.base_url).unwrap();
            client.delete_collection("nope").await.unwrap_err()
        };

        assert!(error.to_string().contains("Failed to delete collection"));
        assert!(format!("{:#}", error).contains("Collection nope does not exist"));

        chroma.stop().await;
    }
}
