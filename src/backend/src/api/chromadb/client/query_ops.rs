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
        .generate_embeddings_with_server(&query_refs, None)
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
    use super::normalize_query_embeddings;
    use crate::api::chromadb::client::ChromaDBClient;
    use crate::api::chromadb::types::QueryRequest;
    use crate::test_support::{
        lock_chroma_endpoint, MockChroma, MockChromaCollection, MockChromaConfig,
    };

    /// Everything past embedding generation needs a real `ollama` binary, so the
    /// tests below only drive the parts of `query_collection` that run before it.
    fn request(collection: &str, query_texts: Vec<&str>) -> QueryRequest {
        QueryRequest {
            collection: collection.to_string(),
            query_texts: query_texts.into_iter().map(str::to_string).collect(),
            n_results: Some(5),
            where_clause: None,
        }
    }

    fn norm(embedding: &[f32]) -> f32 {
        embedding
            .iter()
            .map(|value| value * value)
            .sum::<f32>()
            .sqrt()
    }

    #[test]
    fn test_normalize_query_embeddings_scales_each_vector_to_unit_length() {
        let mut embeddings = vec![vec![3.0, 4.0], vec![0.0, 0.0, 5.0]];

        normalize_query_embeddings(&mut embeddings);

        assert!((embeddings[0][0] - 0.6).abs() < 1e-6);
        assert!((embeddings[0][1] - 0.8).abs() < 1e-6);
        assert!((norm(&embeddings[1]) - 1.0).abs() < 1e-6);
        assert_eq!(embeddings[1][2], 1.0);
    }

    #[test]
    fn test_normalize_query_embeddings_leaves_a_zero_vector_alone() {
        let mut embeddings = vec![vec![0.0, 0.0, 0.0]];

        normalize_query_embeddings(&mut embeddings);

        assert_eq!(embeddings, vec![vec![0.0, 0.0, 0.0]]);
    }

    #[test]
    fn test_normalize_query_embeddings_handles_an_empty_batch() {
        let mut embeddings: Vec<Vec<f32>> = Vec::new();

        normalize_query_embeddings(&mut embeddings);

        assert!(embeddings.is_empty());
    }

    #[actix_web::test]
    async fn test_query_fails_before_embedding_when_the_collection_is_missing() {
        let chroma = MockChroma::start(MockChromaConfig::empty()).await;

        let error = {
            let _guard = lock_chroma_endpoint();
            let client = ChromaDBClient::new(&chroma.base_url).unwrap();
            client
                .query(request("nope", vec!["some text"]), "nomic-embed-text")
                .await
                .unwrap_err()
        };

        assert!(error.to_string().contains("Collection not found"));
        assert!(format!("{:#}", error).contains("Collection nope does not exist"));
        // The lookup is the only thing that happened; no query was attempted.
        let requests = chroma.requests();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].path.ends_with("/collections/nope"));

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_query_rejects_an_empty_query_text_list() {
        let chroma = MockChroma::start(MockChromaConfig::holding(vec![MockChromaCollection::new(
            "notes",
        )]))
        .await;

        let error = {
            let _guard = lock_chroma_endpoint();
            let client = ChromaDBClient::new(&chroma.base_url).unwrap();
            client
                .query(request("notes", vec![]), "nomic-embed-text")
                .await
                .unwrap_err()
        };

        assert_eq!(error.to_string(), "Query texts cannot be empty");
        // The collection was resolved first, then the request was rejected.
        assert_eq!(chroma.requests().len(), 1);

        chroma.stop().await;
    }

    /// The where clause is converted before embeddings are generated, and the
    /// conversion currently drops every clause rather than failing.
    #[actix_web::test]
    async fn test_query_accepts_a_where_clause_without_reaching_the_query_endpoint() {
        let chroma = MockChroma::start(MockChromaConfig::holding(vec![MockChromaCollection::new(
            "notes",
        )]))
        .await;

        let mut query_request = request("notes", vec![]);
        query_request.where_clause = Some(std::collections::HashMap::from([(
            "source".to_string(),
            serde_json::json!("upload"),
        )]));

        let error = {
            let _guard = lock_chroma_endpoint();
            let client = ChromaDBClient::new(&chroma.base_url).unwrap();
            client
                .query(query_request, "nomic-embed-text")
                .await
                .unwrap_err()
        };

        // The clause was silently discarded, so the empty-query-text check is
        // what reports the failure.
        assert_eq!(error.to_string(), "Query texts cannot be empty");

        chroma.stop().await;
    }
}
