//! ChromaDB Client
//!
//! A modular, well-tested client for interacting with ChromaDB.
//!
//! This module is organized by concern:
//! - `ollama.rs`: Ollama server management and embedding generation
//! - `metadata.rs`: Metadata format conversion utilities
//! - `collection_ops.rs`: Collection operations (list, create, get, delete)
//! - `document_ops.rs`: Document operations (add with embeddings)
//! - `query_ops.rs`: Query operations (search with embeddings)

mod collection_ops;
mod document_ops;
mod metadata;
mod ollama;
mod query_ops;
mod where_clause;

#[cfg(test)]
mod tests;

// Re-export for external use
pub use collection_ops::*;
pub use document_ops::*;
pub use query_ops::*;

use crate::api::chromadb::types::{AddDocumentsRequest, Collection, QueryRequest, QueryResponse};
use anyhow::{Context, Result};
use chroma::ChromaHttpClient;

/// Main ChromaDB client
///
/// This client provides a clean interface to ChromaDB operations,
/// with automatic embedding generation using Ollama.
pub struct ChromaDBClient {
    client: ChromaHttpClient,
}

impl ChromaDBClient {
    /// Create a new ChromaDB client
    ///
    /// # Arguments
    /// * `endpoint` - The ChromaDB server endpoint (e.g., "http://localhost:8000")
    ///
    /// # Errors
    /// Returns an error if the ChromaDB server is not accessible or the client cannot be created
    pub fn new(endpoint: &str) -> Result<Self> {
        // Create ChromaHttpClient with the provided endpoint
        // The chroma crate's from_env() reads CHROMA_ENDPOINT, but we set it explicitly
        // Note: set_var is safe here as we're setting it before creating the client
        unsafe {
            std::env::set_var("CHROMA_ENDPOINT", endpoint);
        }
        let client = ChromaHttpClient::from_env().with_context(|| {
            format!(
                "Failed to create ChromaDB client with endpoint: {}. Make sure ChromaDB server is running.",
                endpoint
            )
        })?;

        Ok(Self { client })
    }

    /// Health check - verifies connection to ChromaDB
    ///
    /// # Returns
    /// `true` if the connection is healthy, `false` otherwise
    pub async fn health_check(&self) -> Result<bool> {
        // ChromaDB Rust client doesn't have a direct health check
        // We can test by trying to list collections
        // This will fail if the server is not ready or not accessible
        match self.client.list_collections(10, None).await {
            Ok(_) => {
                println!("✅ ChromaDB health check: Connected");
                Ok(true)
            }
            Err(e) => {
                println!("⚠️ ChromaDB health check failed: {}", e);
                Ok(false)
            }
        }
    }

    /// List all collections
    pub async fn list_collections(&self) -> Result<Vec<Collection>> {
        list_collections(&self.client).await
    }

    /// Create a new collection
    pub async fn create_collection(
        &self,
        name: &str,
        metadata: Option<std::collections::HashMap<String, String>>,
    ) -> Result<Collection> {
        create_collection(&self.client, name, metadata).await
    }

    /// Get a collection by name
    pub async fn get_collection(&self, name: &str) -> Result<Collection> {
        get_collection(&self.client, name).await
    }

    /// Delete a collection by name
    pub async fn delete_collection(&self, name: &str) -> Result<()> {
        delete_collection(&self.client, name).await
    }

    /// Add documents to a collection with automatic embedding generation
    pub async fn add_documents(
        &self,
        request: AddDocumentsRequest,
        embedding_model: &str,
    ) -> Result<()> {
        add_documents(&self.client, request, embedding_model).await
    }

    /// Query a collection with embedding-based search
    pub async fn query(&self, request: QueryRequest, query_model: &str) -> Result<QueryResponse> {
        query_collection(&self.client, request, query_model).await
    }
}
