//! Document operations
//!
//! This module handles adding documents to ChromaDB collections with embedding generation.

use crate::api::chromadb::types::AddDocumentsRequest;
use anyhow::{Context, Result};
use chroma::types::Metadata;
use chroma::ChromaHttpClient;

use super::metadata::vec_to_chromadb_metadata;
use super::ollama::{OllamaConfig, OllamaManager};

/// Normalize embeddings to unit length for cosine similarity
/// This ensures embeddings are on the unit sphere, which is required for proper cosine distance calculation
fn normalize_embeddings(embeddings: &mut [Vec<f32>]) {
    for embedding in embeddings.iter_mut() {
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for value in embedding.iter_mut() {
                *value /= norm;
            }
        }
    }
}

/// Add documents to a collection with automatic embedding generation
pub async fn add_documents(
    client: &ChromaHttpClient,
    request: AddDocumentsRequest,
    embedding_model: &str,
) -> Result<()> {
    let collection = client
        .get_collection(&request.collection)
        .await
        .context("Collection not found")?;

    // Convert metadatas to ChromaDB format
    let metadatas: Option<Vec<Option<Metadata>>> = request.metadatas.map(vec_to_chromadb_metadata);

    println!(
        "🔧 Generating embeddings for {} documents using Ollama embedding model '{}'",
        request.documents.len(),
        embedding_model
    );

    // Generate embeddings using Ollama with configured model
    let config = OllamaConfig {
        model: embedding_model.to_string(),
        ..Default::default()
    };
    let ollama_manager = OllamaManager::new(config);
    let document_refs: Vec<&str> = request.documents.iter().map(|s| s.as_str()).collect();
    let mut embeddings = ollama_manager
        .generate_embeddings_with_server(&document_refs)
        .await
        .with_context(|| {
            format!(
                "Failed to generate embeddings from documents using model '{}'",
                embedding_model
            )
        })?;

    // Log embedding dimension for debugging
    if let Some(first_embedding) = embeddings.first() {
        println!(
            "📐 Document embedding dimension: {} (using model '{}')",
            first_embedding.len(),
            embedding_model
        );
    }

    // Normalize embeddings for cosine similarity (nomic-embed-text should already be normalized,
    // but we ensure it for consistency, especially important for cosine distance metric)
    normalize_embeddings(&mut embeddings);

    // Verify normalization
    if let Some(first_embedding) = embeddings.first() {
        let norm: f32 = first_embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        println!(
            "📊 Document embedding norm after normalization: {:.4} (expected: ~1.0)",
            norm
        );
    }

    // Convert documents to Option<Vec<Option<String>>>
    let docs_vec: Vec<Option<String>> = request.documents.into_iter().map(Some).collect();

    // Use ChromaDB's standard add method with generated embeddings
    // We must batch this to avoid hitting request size limits (e.g. 14k vectors is too big)
    const CHROMA_BATCH_SIZE: usize = 2000;
    let total_docs = request.ids.len();
    let num_batches = total_docs.div_ceil(CHROMA_BATCH_SIZE);

    println!(
        "📦 Uploading {} documents to ChromaDB in {} batches (batch size: {})...",
        total_docs, num_batches, CHROMA_BATCH_SIZE
    );

    // Prepare Vectors for indexing access
    let ids = request.ids;
    // documents and metadatas are already Options wrapping Vecs, unwrapping for slicing if present

    let metas_vec = metadatas.unwrap_or_default();

    // Check consistency
    if ids.len() != embeddings.len() {
        return Err(anyhow::anyhow!(
            "Mismatch between IDs count ({}) and embeddings count ({})",
            ids.len(),
            embeddings.len()
        ));
    }

    for batch_idx in 0..num_batches {
        let start = batch_idx * CHROMA_BATCH_SIZE;
        let end = std::cmp::min((batch_idx + 1) * CHROMA_BATCH_SIZE, total_docs);

        let batch_ids = ids[start..end].to_vec();
        let batch_embeddings = embeddings[start..end].to_vec();

        let batch_documents = if !docs_vec.is_empty() {
            Some(docs_vec[start..end].to_vec())
        } else {
            None
        };

        let batch_metadatas = if !metas_vec.is_empty() {
            Some(metas_vec[start..end].to_vec())
        } else {
            None
        };

        if let Err(e) = collection
            .add(
                batch_ids,
                batch_embeddings,
                batch_documents,
                None, // uris
                batch_metadatas,
            )
            .await
        {
            println!(
                "❌ Failed to add batch {}/{} to ChromaDB: {}",
                batch_idx + 1,
                num_batches,
                e
            );
            return Err(anyhow::anyhow!(
                "Failed to add batch {}/{} to ChromaDB: {}",
                batch_idx + 1,
                num_batches,
                e
            ));
        }

        println!(
            "  ✅ Batch {}/{} added successfully (items {}-{})",
            batch_idx + 1,
            num_batches,
            start,
            end
        );

        // Small delay to be nice to the server
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_function_exists() {
        // Verify the function is defined
    }
}
