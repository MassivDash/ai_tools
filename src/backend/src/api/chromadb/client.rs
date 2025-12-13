use crate::api::chromadb::types::{AddDocumentsRequest, Collection, QueryRequest, QueryResponse};
use anyhow::{Context, Result};
use chroma::{
    embed::{ollama::OllamaEmbeddingFunction, EmbeddingFunction},
    types::{IncludeList, Metadata, MetadataValue, Where},
    ChromaHttpClient,
};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use tokio::time::{sleep, Duration};

pub struct ChromaDBClient {
    client: ChromaHttpClient,
}

impl ChromaDBClient {
    pub fn new(endpoint: &str) -> Result<Self> {
        // Create ChromaHttpClient with the provided endpoint
        // The chroma crate's from_env() reads CHROMA_ENDPOINT, but we set it explicitly
        // Note: set_var is safe here as we're setting it before creating the client
        unsafe {
            std::env::set_var("CHROMA_ENDPOINT", endpoint);
        }
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
        println!(
            "🔧 ChromaDBClient::create_collection called with name: '{}', metadata: {:?}",
            name, metadata
        );

        // Convert HashMap<String, String> to Metadata
        let metadata_map: Option<Metadata> = metadata.map(|m| {
            m.into_iter()
                .map(|(k, v)| (k, MetadataValue::Str(v)))
                .collect()
        });

        println!(
            "🔧 Calling chroma client.create_collection with name: '{}', metadata_map: {:?}",
            name, metadata_map
        );

        let collection = self
            .client
            .create_collection(name, None, metadata_map) // name, schema: None, metadata
            .await
            .with_context(|| format!("Failed to create collection '{}'. Check if collection already exists or if ChromaDB server is accessible.", name))?;

        println!(
            "✅ ChromaDB collection created successfully: {}",
            collection.name()
        );

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

        // ChromaDB standard approach: Generate embeddings from documents using embedding function
        // Use Ollama embedding function (default: http://localhost:11434, model: nomic-embed-text)
        // This is the standard ChromaDB way to handle embeddings when uploading documents
        println!(
            "🔧 Generating embeddings for {} documents using Ollama embedding function",
            request.documents.len()
        );

        // Spawn Ollama server for embedding generation
        println!("🚀 Starting Ollama server for embedding generation...");
        let mut ollama_process_handle = tokio::task::spawn_blocking(|| {
            Command::new("ollama")
                .arg("serve")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
        })
        .await
        .context("Failed to spawn blocking task for Ollama")?
        .context("Failed to spawn Ollama server. Make sure 'ollama' is installed and in PATH.")?;

        // Wait for Ollama to be ready (check if port 11434 is accessible)
        println!("⏳ Waiting for Ollama server to be ready...");
        let mut retries = 30; // Wait up to 30 seconds
        let mut ollama_ready = false;
        while retries > 0 {
            if let Ok(_) = tokio::net::TcpStream::connect("127.0.0.1:11434").await {
                ollama_ready = true;
                break;
            }
            sleep(Duration::from_millis(1000)).await;
            retries -= 1;
        }

        if !ollama_ready {
            let _ = tokio::task::spawn_blocking(move || {
                let _ = ollama_process_handle.kill();
                let _ = ollama_process_handle.wait();
            })
            .await;
            return Err(anyhow::anyhow!("Ollama server failed to start within 30 seconds. Make sure Ollama is properly installed."));
        }

        println!("✅ Ollama server is ready");

        // Give Ollama a moment to fully initialize after port is open
        sleep(Duration::from_millis(500)).await;

        // Check if model is available, pull if needed
        let model_name = "nomic-embed-text";
        println!("🔍 Checking if model '{}' is available...", model_name);

        // Try to pull the model if it's not available (this will be a no-op if already present)
        let model_name_clone = model_name.to_string();
        match tokio::task::spawn_blocking(move || {
            Command::new("ollama")
                .arg("pull")
                .arg(&model_name_clone)
                .output()
        })
        .await
        {
            Ok(Ok(output)) => {
                if output.status.success() {
                    println!("✅ Model '{}' is available", model_name);
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    println!("⚠️ Warning: Model pull may have failed: {}", stderr);
                    // Continue anyway - model might already be available
                }
            }
            Ok(Err(e)) => {
                println!("⚠️ Warning: Failed to execute ollama pull: {}", e);
                // Continue anyway - model might already be available
            }
            Err(e) => {
                println!("⚠️ Warning: Failed to spawn model pull task: {}", e);
                // Continue anyway - model might already be available
            }
        }

        // Generate embeddings
        let embeddings = {
            // Use default Ollama host and model
            println!(
                "🔧 Initializing Ollama embedding function with model '{}'...",
                model_name
            );
            let embedding_fn_result =
                OllamaEmbeddingFunction::new("http://localhost:11434", model_name).await;

            let embedding_fn = match embedding_fn_result {
                Ok(fn_) => fn_,
                Err(e) => {
                    let error_msg = format!("Failed to initialize Ollama embedding function: {:?}. Make sure the model '{}' is available. You may need to run 'ollama pull {}' first.", e, model_name, model_name);
                    println!("❌ {}", error_msg);
                    return Err(anyhow::anyhow!(error_msg));
                }
            };

            // Convert Vec<String> to Vec<&str> for embed_strs method
            let document_refs: Vec<&str> = request.documents.iter().map(|s| s.as_str()).collect();
            embedding_fn
                .embed_strs(&document_refs)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to generate embeddings: {}", e))
                .context("Failed to generate embeddings from documents")?
        };

        println!(
            "✅ Generated {} embeddings (dimension: {})",
            embeddings.len(),
            embeddings.first().map(|e| e.len()).unwrap_or(0)
        );

        // Kill Ollama server after embeddings are generated
        println!("🛑 Stopping Ollama server...");
        let kill_result = tokio::task::spawn_blocking(move || match ollama_process_handle.kill() {
            Ok(_) => {
                let _ = ollama_process_handle.wait();
                Ok(())
            }
            Err(e) => Err(e),
        })
        .await;

        match kill_result {
            Ok(Ok(_)) => {
                println!("✅ Ollama server stopped successfully");
            }
            Ok(Err(e)) => {
                println!("⚠️ Warning: Failed to stop Ollama server: {}", e);
            }
            Err(e) => {
                println!("⚠️ Warning: Failed to wait for Ollama kill task: {}", e);
            }
        }

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

        // Generate embeddings from query texts using the same embedding function as documents
        println!(
            "🔍 Generating embeddings for query: {:?}",
            request.query_texts
        );

        // Spawn Ollama server for embedding generation
        println!("🚀 Starting Ollama server for query embedding generation...");
        let mut ollama_process_handle = tokio::task::spawn_blocking(|| {
            Command::new("ollama")
                .arg("serve")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
        })
        .await
        .context("Failed to spawn blocking task for Ollama")?
        .context("Failed to spawn Ollama server. Make sure 'ollama' is installed and in PATH.")?;

        // Wait for Ollama to be ready (check if port 11434 is accessible)
        println!("⏳ Waiting for Ollama server to be ready...");
        let mut retries = 30; // Wait up to 30 seconds
        let mut ollama_ready = false;
        while retries > 0 {
            if let Ok(_) = tokio::net::TcpStream::connect("127.0.0.1:11434").await {
                ollama_ready = true;
                break;
            }
            sleep(Duration::from_millis(1000)).await;
            retries -= 1;
        }

        if !ollama_ready {
            let _ = tokio::task::spawn_blocking(move || {
                let _ = ollama_process_handle.kill();
                let _ = ollama_process_handle.wait();
            })
            .await;
            return Err(anyhow::anyhow!("Ollama server failed to start within 30 seconds. Make sure Ollama is properly installed."));
        }

        println!("✅ Ollama server is ready");

        // Give Ollama a moment to fully initialize after port is open
        sleep(Duration::from_millis(500)).await;

        // Check if model is available, pull if needed
        let model_name = "nomic-embed-text";
        println!("🔍 Checking if model '{}' is available...", model_name);

        // Try to pull the model if it's not available (this will be a no-op if already present)
        let model_name_clone = model_name.to_string();
        match tokio::task::spawn_blocking(move || {
            Command::new("ollama")
                .arg("pull")
                .arg(&model_name_clone)
                .output()
        })
        .await
        {
            Ok(Ok(output)) => {
                if output.status.success() {
                    println!("✅ Model '{}' is available", model_name);
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    println!("⚠️ Warning: Model pull may have failed: {}", stderr);
                    // Continue anyway - model might already be available
                }
            }
            Ok(Err(e)) => {
                println!("⚠️ Warning: Failed to execute ollama pull: {}", e);
                // Continue anyway - model might already be available
            }
            Err(e) => {
                println!("⚠️ Warning: Failed to spawn model pull task: {}", e);
                // Continue anyway - model might already be available
            }
        }

        // Generate query embeddings
        let query_embeddings: Vec<Vec<f32>> = {
            println!(
                "🔧 Initializing Ollama embedding function with model '{}'...",
                model_name
            );
            let embedding_fn_result =
                OllamaEmbeddingFunction::new("http://localhost:11434", model_name).await;

            let embedding_fn = match embedding_fn_result {
                Ok(fn_) => fn_,
                Err(e) => {
                    let error_msg = format!("Failed to initialize Ollama embedding function: {:?}. Make sure the model '{}' is available. You may need to run 'ollama pull {}' first.", e, model_name, model_name);
                    println!("❌ {}", error_msg);
                    // Kill Ollama before returning error
                    let _ = tokio::task::spawn_blocking(move || {
                        let _ = ollama_process_handle.kill();
                        let _ = ollama_process_handle.wait();
                    })
                    .await;
                    return Err(anyhow::anyhow!(error_msg));
                }
            };

            // Convert Vec<String> to Vec<&str> for embed_strs method
            let query_refs: Vec<&str> = request.query_texts.iter().map(|s| s.as_str()).collect();
            let embeddings = embedding_fn
                .embed_strs(&query_refs)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to generate query embeddings: {}", e))
                .context("Failed to generate embeddings from query texts")?;

            println!(
                "✅ Generated {} query embeddings (dimension: {})",
                embeddings.len(),
                embeddings.first().map(|e| e.len()).unwrap_or(0)
            );

            embeddings
        };

        // Kill Ollama server after embeddings are generated
        println!("🛑 Stopping Ollama server...");
        let kill_result = tokio::task::spawn_blocking(move || match ollama_process_handle.kill() {
            Ok(_) => {
                let _ = ollama_process_handle.wait();
                Ok(())
            }
            Err(e) => Err(e),
        })
        .await;

        match kill_result {
            Ok(Ok(_)) => {
                println!("✅ Ollama server stopped successfully");
            }
            Ok(Err(e)) => {
                println!("⚠️ Warning: Failed to stop Ollama server: {}", e);
            }
            Err(e) => {
                println!("⚠️ Warning: Failed to wait for Ollama kill task: {}", e);
            }
        }

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
