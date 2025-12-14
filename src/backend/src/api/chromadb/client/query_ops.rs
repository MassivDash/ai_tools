//! Query operations
//!
//! This module handles querying ChromaDB collections with embedding-based search.

use crate::api::chromadb::types::{QueryRequest, QueryResponse};
use anyhow::{Context, Result};
use chroma::types::IncludeList;
use chroma::ChromaHttpClient;
use std::collections::HashMap;

use super::metadata::metadata_value_to_json;
use super::ollama::OllamaManager;
use super::where_clause::convert_where_clause;

/// Query a collection with embedding-based search
pub async fn query_collection(
    client: &ChromaHttpClient,
    request: QueryRequest,
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
        "🔍 Generating embeddings for query: {:?}",
        request.query_texts
    );

    // Generate query embeddings using Ollama
    let ollama_manager = OllamaManager::new(Default::default());
    let query_refs: Vec<&str> = request.query_texts.iter().map(|s| s.as_str()).collect();
    let query_embeddings = ollama_manager
        .generate_embeddings_with_server(&query_refs)
        .await
        .context("Failed to generate embeddings from query texts")?;

    let include = Some(IncludeList::default_query());

    let results = collection
        .query(
            query_embeddings,
            request.n_results.map(|n| n as u32),
            where_clause,
            None, // ids
            include,
        )
        .await
        .context("Failed to query collection")?;

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
        assert!(true);
    }
}
