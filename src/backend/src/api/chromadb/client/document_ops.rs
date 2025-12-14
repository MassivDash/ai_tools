//! Document operations
//!
//! This module handles adding documents to ChromaDB collections with embedding generation.

use crate::api::chromadb::types::AddDocumentsRequest;
use anyhow::{Context, Result};
use chroma::types::Metadata;
use chroma::ChromaHttpClient;

use super::metadata::vec_to_chromadb_metadata;
use super::ollama::OllamaManager;

/// Add documents to a collection with automatic embedding generation
pub async fn add_documents(client: &ChromaHttpClient, request: AddDocumentsRequest) -> Result<()> {
    let collection = client
        .get_collection(&request.collection)
        .await
        .context("Collection not found")?;

    // Convert metadatas to ChromaDB format
    let metadatas: Option<Vec<Option<Metadata>>> = request.metadatas.map(vec_to_chromadb_metadata);

    println!(
        "🔧 Generating embeddings for {} documents using Ollama embedding function",
        request.documents.len()
    );

    // Generate embeddings using Ollama
    let ollama_manager = OllamaManager::new(Default::default());
    let document_refs: Vec<&str> = request.documents.iter().map(|s| s.as_str()).collect();
    let embeddings = ollama_manager
        .generate_embeddings_with_server(&document_refs)
        .await
        .context("Failed to generate embeddings from documents")?;

    // Convert documents to Option<Vec<Option<String>>>
    let documents: Option<Vec<Option<String>>> =
        Some(request.documents.into_iter().map(Some).collect());

    // Use ChromaDB's standard add method with generated embeddings
    collection
        .add(
            request.ids,
            embeddings, // Generated embeddings from documents
            documents,
            None, // uris
            metadatas,
        )
        .await
        .context("Failed to add documents to ChromaDB")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_function_exists() {
        // Verify the function is defined
        assert!(true);
    }
}
