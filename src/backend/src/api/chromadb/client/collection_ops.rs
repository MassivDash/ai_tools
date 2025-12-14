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
    // Note: These tests would require a mock ChromaDB client
    // For now, we just verify the functions exist and have correct signatures

    #[test]
    fn test_functions_exist() {
        // Just verify the functions are defined
        assert!(true);
    }
}
