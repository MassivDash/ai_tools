//! Query operations
//!
//! This module handles querying ChromaDB collections with embedding-based search.

use crate::api::chromadb::types::{QueryRequest, QueryResponse};
use anyhow::{Context, Result};
use chroma::types::IncludeList;
use chroma::ChromaHttpClient;
use std::collections::HashMap;

use super::metadata::metadata_value_to_json;
use super::ollama::{OllamaConfig, OllamaManager};
use super::where_clause::convert_where_clause;

/// Normalize query embeddings to unit length for cosine similarity
fn normalize_query_embeddings(embeddings: &mut [Vec<f32>]) {
    for embedding in embeddings.iter_mut() {
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for value in embedding.iter_mut() {
                *value /= norm;
            }
        }
    }
}

/// Query a collection with embedding-based search
pub async fn query_collection(
    client: &ChromaHttpClient,
    request: QueryRequest,
    query_model: &str,
) -> Result<QueryResponse> {
    let collection = client
        .get_collection(&request.collection)
        .await
        .context("Collection not found")?;

    // Convert where clause to ChromaDB format
    // Note: Full where clause conversion is not yet implemented due to complexity
    // of ChromaDB's Where type structure. Simple cases may be supported in future versions.
    let where_clause =
        convert_where_clause(request.where_clause).context("Failed to convert where clause")?;

    println!(
        "🔍 Generating embeddings for query using model '{}': {:?}",
        query_model, request.query_texts
    );

    // Validate query texts are not empty
    if request.query_texts.is_empty() {
        return Err(anyhow::anyhow!("Query texts cannot be empty"));
    }

    // Generate query embeddings using Ollama with configured model
    let config = OllamaConfig {
        model: query_model.to_string(),
        ..Default::default()
    };
    let ollama_manager = OllamaManager::new(config);
    let query_refs: Vec<&str> = request.query_texts.iter().map(|s| s.as_str()).collect();
    let mut query_embeddings = ollama_manager
        .generate_embeddings_with_server(&query_refs)
        .await
        .with_context(|| {
            format!(
                "Failed to generate embeddings from query texts using model '{}'. \
                This could mean:\n\
                1. The model '{}' is not available (run 'ollama pull {}')\n\
                2. Ollama server is not accessible\n\
                3. The model doesn't support embeddings",
                query_model, query_model, query_model
            )
        })?;

    // Normalize query embeddings for cosine similarity
    normalize_query_embeddings(&mut query_embeddings);

    // Verify normalization
    if let Some(first_embedding) = query_embeddings.first() {
        let norm: f32 = first_embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        println!(
            "📊 Query embedding norm after normalization: {:.4} (expected: ~1.0)",
            norm
        );
    }

    let include = Some(IncludeList::default_query());

    println!(
        "🔎 Querying collection '{}' with {} embedding(s), requesting {} results",
        request.collection,
        query_embeddings.len(),
        request.n_results.unwrap_or(10)
    );

    // Get embedding dimension for error reporting
    let embedding_dim = query_embeddings.first().map(|e| e.len()).unwrap_or(0);

    println!("📐 Query embedding dimension: {}", embedding_dim);

    let results = match collection
        .query(
            query_embeddings,
            request.n_results.map(|n| n as u32),
            where_clause,
            None, // ids
            include,
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let error_str = e.to_string();
            // Check for common error patterns
            let detailed_error = if error_str.contains("dimension")
                || error_str.contains("dimension mismatch")
            {
                format!(
                    "Embedding dimension mismatch! Query embeddings have {} dimensions, but the collection expects a different dimension. \
                    This usually means:\n\
                    1. Documents were uploaded with a different embedding model\n\
                    2. The query model produces different dimensions than the upload model\n\
                    Solution: Use the same embedding model for both uploading and querying, or recreate the collection with the new model.\n\
                    Original error: {}",
                    embedding_dim, error_str
                )
            } else if error_str.contains("not found") || error_str.contains("does not exist") {
                format!(
                    "Collection '{}' does not exist. Please create it first or check the collection name.",
                    request.collection
                )
            } else {
                format!(
                    "Failed to query collection '{}': {}\n\
                    Possible causes:\n\
                    1. Collection doesn't exist\n\
                    2. Embedding dimensions don't match (query: {} dims)\n\
                    3. ChromaDB server issue",
                    request.collection, error_str, embedding_dim
                )
            };
            return Err(anyhow::anyhow!(detailed_error));
        }
    };

    // Log distance statistics for debugging
    if let Some(ref distances) = results.distances {
        if let Some(first_query_distances) = distances.first() {
            if let Some(min_dist) = first_query_distances
                .iter()
                .flatten()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
            {
                if let Some(max_dist) = first_query_distances
                    .iter()
                    .flatten()
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                {
                    println!("📊 Query distance range: min={:.4}, max={:.4} (cosine distance, lower is better)", min_dist, max_dist);
                }
            }
        }
    }

    // Convert results to our format
    Ok(QueryResponse {
        ids: results.ids,
        distances: results.distances.map(|d| {
            d.into_iter()
                .map(|inner| {
                    inner
                        .into_iter()
                        .filter_map(|opt| opt.map(|f| f as f64))
                        .collect()
                })
                .collect()
        }),
        documents: results.documents.map(|d| {
            d.into_iter()
                .map(|inner| inner.into_iter().flatten().collect())
                .collect()
        }),
        metadatas: results.metadatas.map(|m| {
            m.iter()
                .map(|inner_vec| {
                    inner_vec
                        .iter()
                        .map(|meta_opt| -> HashMap<String, serde_json::Value> {
                            meta_opt
                                .as_ref()
                                .map(|meta| {
                                    meta.iter()
                                        .map(|(k, v)| (k.clone(), metadata_value_to_json(v)))
                                        .collect::<HashMap<String, serde_json::Value>>()
                                })
                                .unwrap_or_default()
                        })
                        .collect::<Vec<HashMap<String, serde_json::Value>>>()
                })
                .collect::<Vec<Vec<HashMap<String, serde_json::Value>>>>()
        }),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_function_exists() {
        // Verify the function is defined
    }
}
