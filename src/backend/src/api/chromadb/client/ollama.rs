//! Ollama server management and embedding generation
//!
//! This module handles starting/stopping Ollama server and generating embeddings
//! using the nomic-embed-text model.

use anyhow::{Context, Result};
use chroma::embed::{ollama::OllamaEmbeddingFunction, EmbeddingFunction};
use std::process::{Command, Stdio};
use tokio::time::{sleep, Duration};

/// Configuration for Ollama embedding generation
pub struct OllamaConfig {
    pub host: String,
    pub model: String,
    pub port: u16,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            host: "http://localhost".to_string(),
            model: "nomic-embed-text".to_string(),
            port: 11434,
            max_retries: 30,
            retry_delay_ms: 1000,
        }
    }
}

/// Manages Ollama server lifecycle and embedding generation
pub struct OllamaManager {
    config: OllamaConfig,
}

impl OllamaManager {
    pub fn new(config: OllamaConfig) -> Self {
        Self { config }
    }

    /// Start Ollama server and wait for it to be ready
    pub async fn start_server(&self) -> Result<std::process::Child> {
        println!("🚀 Starting Ollama server for embedding generation...");

        let process = tokio::task::spawn_blocking(|| {
            Command::new("ollama")
                .arg("serve")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
        })
        .await
        .context("Failed to spawn blocking task for Ollama")?
        .context("Failed to spawn Ollama server. Make sure 'ollama' is installed and in PATH.")?;

        // Wait for Ollama to be ready
        self.wait_for_server().await?;

        println!("✅ Ollama server is ready");

        // Give Ollama a moment to fully initialize after port is open
        sleep(Duration::from_millis(500)).await;

        Ok(process)
    }

    /// Wait for Ollama server to be ready by checking if port is accessible
    async fn wait_for_server(&self) -> Result<()> {
        println!("⏳ Waiting for Ollama server to be ready...");

        let mut retries = self.config.max_retries;
        while retries > 0 {
            if tokio::net::TcpStream::connect(format!("127.0.0.1:{}", self.config.port))
                .await
                .is_ok()
            {
                return Ok(());
            }
            sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
            retries -= 1;
        }

        Err(anyhow::anyhow!(
            "Ollama server failed to start within {} seconds. Make sure Ollama is properly installed.",
            self.config.max_retries
        ))
    }

    /// Ensure the embedding model is available (pull if needed)
    pub async fn ensure_model_available(&self) -> Result<()> {
        println!(
            "🔍 Checking if model '{}' is available...",
            self.config.model
        );

        let model_name = self.config.model.clone();
        match tokio::task::spawn_blocking(move || {
            Command::new("ollama").arg("pull").arg(&model_name).output()
        })
        .await
        {
            Ok(Ok(output)) => {
                if output.status.success() {
                    println!("✅ Model '{}' is available", self.config.model);
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

        Ok(())
    }

    /// Stop the Ollama server process
    pub async fn stop_server(&self, mut process: std::process::Child) {
        println!("🛑 Stopping Ollama server...");

        let kill_result = tokio::task::spawn_blocking(move || {
            let _ = process.kill();
            process.wait()
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
    }

    /// Generate embeddings for the given texts
    pub async fn generate_embeddings(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        println!(
            "🔧 Initializing Ollama embedding function with model '{}'...",
            self.config.model
        );

        let endpoint = format!("{}:{}", self.config.host, self.config.port);
        let embedding_fn = OllamaEmbeddingFunction::new(&endpoint, &self.config.model)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to initialize Ollama embedding function: {:?}. Make sure the model '{}' is available. You may need to run 'ollama pull {}' first.",
                    e,
                    self.config.model,
                    self.config.model
                )
            })?;

        let embeddings = embedding_fn
            .embed_strs(texts)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to generate embeddings: {}", e))
            .context("Failed to generate embeddings")?;

        println!(
            "✅ Generated {} embeddings (dimension: {})",
            embeddings.len(),
            embeddings.first().map(|e| e.len()).unwrap_or(0)
        );

        Ok(embeddings)
    }

    /// Complete workflow: start server, ensure model, generate embeddings, stop server
    pub async fn generate_embeddings_with_server(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let process = self.start_server().await?;

        // Ensure model is available (non-blocking if already present)
        self.ensure_model_available().await?;

        // Generate embeddings
        let result = self.generate_embeddings(texts).await;

        // Always stop the server, even if embedding generation failed
        self.stop_server(process).await;

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_config_default() {
        let config = OllamaConfig::default();
        assert_eq!(config.host, "http://localhost");
        assert_eq!(config.model, "nomic-embed-text");
        assert_eq!(config.port, 11434);
        assert_eq!(config.max_retries, 30);
    }

    #[test]
    fn test_ollama_manager_creation() {
        let config = OllamaConfig::default();
        let _manager = OllamaManager::new(config);
        // Just verify it can be created
        assert!(true);
    }
}
