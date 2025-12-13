use crate::api::chromadb::types::{AddDocumentsRequest, Collection, QueryRequest, QueryResponse};
use anyhow::{Context, Result};
use chroma::{
    types::{IncludeList, Metadata, MetadataValue, Where},
    ChromaHttpClient,
};
use std::collections::HashMap;

pub struct ChromaDBClient {
    client: ChromaHttpClient,
}

impl ChromaDBClient {
    pub fn new(endpoint: &str) -> Result<Self> {
        // Create ChromaHttpClient with the provided endpoint
        // The chroma crate's from_env() reads CHROMA_ENDPOINT, but we set it explicitly
        std::env::set_var("CHROMA_ENDPOINT", endpoint);
        let client = ChromaHttpClient::from_env()
            .context(format!("Failed to create ChromaDB client with endpoint: {}. Make sure ChromaDB server is running.", endpoint))?;

        Ok(Self { client })
    }

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

    pub async fn list_collections(&self) -> Result<Vec<Collection>> {
        let collections = self
            .client
            .list_collections(100, None) // limit: 100, offset: None
            .await
            .context("Failed to list collections")?;

        let mut result = Vec::new();
        for collection in collections {
            // Get count properly
            let actual_count = if let Ok(col) = self.client.get_collection(collection.name()).await
            {
                col.count().await.unwrap_or(0) as usize
            } else {
                0
            };

            result.push(Collection {
                id: collection.id().to_string(),
                name: collection.name().to_string(),
                metadata: collection.metadata().as_ref().map(|m| {
                    m.iter()
                        .map(|(k, v)| {
                            // Convert MetadataValue to String
                            let val_str = match v {
                                MetadataValue::Str(s) => s.clone(),
                                MetadataValue::Int(i) => i.to_string(),
                                MetadataValue::Float(f) => f.to_string(),
                                MetadataValue::Bool(b) => b.to_string(),
                                MetadataValue::SparseVector(_) => "SparseVector".to_string(),
                            };
                            (k.clone(), val_str)
                        })
                        .collect()
                }),
                count: Some(actual_count),
            });
        }

        Ok(result)
    }

    pub async fn create_collection(
        &self,
        name: &str,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<Collection> {
        println!("🔧 ChromaDBClient::create_collection called with name: '{}', metadata: {:?}", name, metadata);
        
        // Convert HashMap<String, String> to Metadata
        let metadata_map: Option<Metadata> = metadata.map(|m| {
            m.into_iter()
                .map(|(k, v)| (k, MetadataValue::Str(v)))
                .collect()
        });

        println!("🔧 Calling chroma client.create_collection with name: '{}', metadata_map: {:?}", name, metadata_map);
        
        let collection = self
            .client
            .create_collection(name, None, metadata_map) // name, schema: None, metadata
            .await
            .with_context(|| format!("Failed to create collection '{}'. Check if collection already exists or if ChromaDB server is accessible.", name))?;
        
        println!("✅ ChromaDB collection created successfully: {}", collection.name());

        Ok(Collection {
            id: collection.id().to_string(),
            name: collection.name().to_string(),
            metadata: collection.metadata().as_ref().map(|m| {
                m.iter()
                    .map(|(k, v)| {
                        let val_str = match v {
                            MetadataValue::Str(s) => s.clone(),
                            MetadataValue::Int(i) => i.to_string(),
                            MetadataValue::Float(f) => f.to_string(),
                            MetadataValue::Bool(b) => b.to_string(),
                            MetadataValue::SparseVector(_) => "SparseVector".to_string(),
                        };
                        (k.clone(), val_str)
                    })
                    .collect()
            }),
            count: Some(0),
        })
    }

    pub async fn get_collection(&self, name: &str) -> Result<Collection> {
        let collection = self
            .client
            .get_collection(name)
            .await
            .context("Failed to get collection")?;

        let count = collection.count().await.unwrap_or(0) as usize;

        Ok(Collection {
            id: collection.id().to_string(),
            name: collection.name().to_string(),
            metadata: collection.metadata().as_ref().map(|m| {
                m.iter()
                    .map(|(k, v)| {
                        let val_str = match v {
                            MetadataValue::Str(s) => s.clone(),
                            MetadataValue::Int(i) => i.to_string(),
                            MetadataValue::Float(f) => f.to_string(),
                            MetadataValue::Bool(b) => b.to_string(),
                            MetadataValue::SparseVector(_) => "SparseVector".to_string(),
                        };
                        (k.clone(), val_str)
                    })
                    .collect()
            }),
            count: Some(count),
        })
    }

    pub async fn delete_collection(&self, name: &str) -> Result<()> {
        self.client
            .delete_collection(name)
            .await
            .context("Failed to delete collection")?;
        Ok(())
    }

    pub async fn add_documents(&self, request: AddDocumentsRequest) -> Result<()> {
        let collection = self
            .client
            .get_collection(&request.collection)
            .await
            .context("Collection not found")?;

        // Convert metadatas to ChromaDB format
        // Note: ChromaDB requires embeddings, but we can pass empty vec and let it generate
        // For now, we'll need to generate embeddings or pass empty
        let metadatas: Option<Vec<Option<Metadata>>> = request.metadatas.map(|m| {
            m.into_iter()
                .map(|meta| {
                    Some(
                        meta.into_iter()
                            .map(|(k, v)| (k, MetadataValue::Str(v)))
                            .collect(),
                    )
                })
                .collect()
        });

        // Convert documents to Option<Vec<Option<String>>>
        let documents: Option<Vec<Option<String>>> =
            Some(request.documents.into_iter().map(Some).collect());

        // For now, we need embeddings. We'll pass empty and ChromaDB should generate them
        // But the API requires embeddings, so we need to handle this differently
        // Let's use empty embeddings for now - this might need embedding generation
        let empty_embeddings: Vec<Vec<f32>> = vec![];

        collection
            .add(
                request.ids,
                empty_embeddings, // embeddings - required by API
                documents,
                None, // uris
                metadatas,
            )
            .await
            .context("Failed to add documents. Note: Embeddings are required. You may need to generate them first.")?;

        Ok(())
    }

    pub async fn query(&self, request: QueryRequest) -> Result<QueryResponse> {
        let collection = self
            .client
            .get_collection(&request.collection)
            .await
            .context("Collection not found")?;

        // Convert where clause to ChromaDB format
        // For now, we'll skip where clause conversion as it requires proper Where structure
        // TODO: Implement proper Where clause conversion from JSON
        let where_clause: Option<Where> = None;

        // For query, we need query_embeddings, not query_texts
        // We'll need to generate embeddings from query_texts
        // For now, let's use empty embeddings - this is a limitation
        // In production, you'd generate embeddings from the query texts
        let query_embeddings: Vec<Vec<f32>> = vec![];

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
            .context("Failed to query collection. Note: Query embeddings are required.")?;

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
                    .map(|inner| inner.into_iter().filter_map(|opt| opt).collect())
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
                                            .map(|(k, v)| {
                                                // Convert MetadataValue to serde_json::Value
                                                let json_val: serde_json::Value = match v {
                                                    MetadataValue::Str(s) => {
                                                        serde_json::Value::String(s.clone())
                                                    }
                                                    MetadataValue::Int(i) => {
                                                        serde_json::Value::Number((*i).into())
                                                    }
                                                    MetadataValue::Float(f) => {
                                                        serde_json::Value::Number(
                                                            serde_json::Number::from_f64(*f)
                                                                .unwrap_or(
                                                                    serde_json::Number::from(0),
                                                                ),
                                                        )
                                                    }
                                                    MetadataValue::Bool(b) => {
                                                        serde_json::Value::Bool(*b)
                                                    }
                                                    MetadataValue::SparseVector(_) => {
                                                        serde_json::Value::String(
                                                            "SparseVector".to_string(),
                                                        )
                                                    }
                                                };
                                                (k.clone(), json_val)
                                            })
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
}
