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
    /// Returns Ok(Some(process)) if we started it, Ok(None) if it was already running
    pub async fn start_server(&self) -> Result<Option<std::process::Child>> {
        // First check if Ollama is already running
        if tokio::net::TcpStream::connect(format!("127.0.0.1:{}", self.config.port))
            .await
            .is_ok()
        {
            println!(
                "✅ Ollama server is already running on port {}",
                self.config.port
            );
            return Ok(None);
        }

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

        Ok(Some(process))
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
    /// This function will:
    /// 1. Check if the model is already available
    /// 2. If not, pull it and wait for completion
    /// 3. Verify the model is available before returning
    pub async fn ensure_model_available(&self) -> Result<()> {
        println!(
            "🔍 Checking if model '{}' is available...",
            self.config.model
        );

        // First, check if model is already available
        // Note: Model names in ollama list might include tags like ":latest"
        // So we check if the model name starts with our model name
        let model_name = self.config.model.clone();
        let is_available = tokio::task::spawn_blocking({
            let model_name = model_name.clone();
            move || {
                let output = Command::new("ollama").arg("list").output();
                match output {
                    Ok(output) if output.status.success() => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        // Check if model name appears in the list (with or without tag)
                        // Model names in ollama list are like "model-name:tag" or just "model-name"
                        stdout.lines().skip(1).any(|line| {
                            if let Some(first_word) = line.split_whitespace().next() {
                                // Check if it matches exactly or starts with model name followed by colon
                                first_word == model_name
                                    || first_word.starts_with(&format!("{}:", model_name))
                            } else {
                                false
                            }
                        })
                    }
                    _ => false,
                }
            }
        })
        .await
        .map_err(|e| anyhow::anyhow!("Failed to check model availability: {}", e))?;

        if is_available {
            println!("✅ Model '{}' is already available", self.config.model);
            return Ok(());
        }

        // Model not available, pull it
        println!(
            "📥 Pulling model '{}' (this may take a while)...",
            self.config.model
        );
        let model_name = self.config.model.clone();
        let pull_result = tokio::task::spawn_blocking(move || {
            Command::new("ollama").arg("pull").arg(&model_name).output()
        })
        .await
        .map_err(|e| anyhow::anyhow!("Failed to spawn model pull task: {}", e))?;

        match pull_result {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    println!("✅ Model '{}' pulled successfully", self.config.model);
                    // Print pull output for visibility
                    if !stdout.trim().is_empty() {
                        println!("📋 Pull output: {}", stdout);
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let error_msg = format!(
                        "Failed to pull model '{}'. stderr: {}, stdout: {}",
                        self.config.model, stderr, stdout
                    );
                    println!("{}", error_msg);
                    return Err(anyhow::anyhow!(error_msg));
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to execute ollama pull: {}", e);
                println!("{}", error_msg);
                return Err(anyhow::anyhow!(error_msg));
            }
        }

        // Verify the model is now available
        // Note: Model names in ollama list might include tags like ":latest"
        let model_name = self.config.model.clone();
        let is_now_available = tokio::task::spawn_blocking({
            let model_name = model_name.clone();
            move || {
                let output = Command::new("ollama").arg("list").output();
                match output {
                    Ok(output) if output.status.success() => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        // Check if model name appears in the list (with or without tag)
                        stdout.lines().skip(1).any(|line| {
                            if let Some(first_word) = line.split_whitespace().next() {
                                first_word == model_name
                                    || first_word.starts_with(&format!("{}:", model_name))
                            } else {
                                false
                            }
                        })
                    }
                    _ => false,
                }
            }
        })
        .await
        .map_err(|e| anyhow::anyhow!("Failed to verify model availability: {}", e))?;

        if !is_now_available {
            return Err(anyhow::anyhow!(
                "Model '{}' was pulled but is not showing up in 'ollama list'. Please check manually.",
                self.config.model
            ));
        }

        println!(
            "✅ Model '{}' is now available and verified",
            self.config.model
        );
        Ok(())
    }

    /// Stop the Ollama server process
    /// Only stops if we started it (process is Some)
    pub async fn stop_server(&self, process: Option<std::process::Child>) {
        if let Some(mut process) = process {
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
        } else {
            println!("ℹ️  Ollama server was already running, not stopping it");
        }
    }

    /// Generate embeddings for the given texts
    pub async fn generate_embeddings(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        println!(
            "🔧 Initializing Ollama embedding function with model '{}' at {}:{}...",
            self.config.model, self.config.host, self.config.port
        );

        // Verify the model name matches what we expect (check for common issues)
        if self.config.model.contains(":latest") {
            println!(
                "ℹ️  Model name includes ':latest' tag: '{}'",
                self.config.model
            );
        }

        let endpoint = format!("{}:{}", self.config.host, self.config.port);

        // Try to initialize the embedding function with detailed error handling
        let embedding_fn = match OllamaEmbeddingFunction::new(&endpoint, &self.config.model).await {
            Ok(fn_) => {
                println!("✅ Ollama embedding function initialized successfully");
                fn_
            }
            Err(e) => {
                let error_msg = format!(
                    "Failed to initialize Ollama embedding function with model '{}' at endpoint '{}': {:?}\n\
                    Troubleshooting:\n\
                    1. Make sure Ollama server is running (check with 'ollama list')\n\
                    2. Verify the model '{}' exists (run 'ollama list' to see available models)\n\
                    3. If the model doesn't exist, run 'ollama pull {}'\n\
                    4. Check that Ollama is accessible at {}:{}",
                    self.config.model, endpoint, e, self.config.model, self.config.model, self.config.host, self.config.port
                );
                println!("{}", error_msg);
                return Err(anyhow::anyhow!(error_msg));
            }
        };

        println!("📝 Generating embeddings for {} text(s)...", texts.len());

        let embeddings = match embedding_fn.embed_strs(texts).await {
            Ok(embeds) => embeds,
            Err(e) => {
                let error_msg = format!(
                    "Failed to generate embeddings using model '{}': {}\n\
                    This could mean:\n\
                    1. The model '{}' doesn't support embeddings\n\
                    2. The model is corrupted or incomplete\n\
                    3. There's a network issue connecting to Ollama",
                    self.config.model, e, self.config.model
                );
                println!("{}", error_msg);
                return Err(anyhow::anyhow!(error_msg));
            }
        };

        let embedding_dim = embeddings.first().map(|e| e.len()).unwrap_or(0);
        println!(
            "✅ Generated {} embeddings using model '{}' (dimension: {})",
            embeddings.len(),
            self.config.model,
            embedding_dim
        );

        // Log expected dimensions for common models to help debug mismatches
        match embedding_dim {
            384 => println!(
                "ℹ️  Dimension 384 typically indicates: chroma/all-minilm-l6-v2-f32 or similar"
            ),
            768 => println!("ℹ️  Dimension 768 typically indicates: nomic-embed-text"),
            1024 => {
                println!("ℹ️  Dimension 1024 typically indicates: mxbai-embed-large or similar")
            }
            _ => println!(
                "ℹ️  Dimension {} - verify this matches your model's expected output",
                embedding_dim
            ),
        }

        Ok(embeddings)
    }

    /// Complete workflow: start server, ensure model, generate embeddings, stop server
    pub async fn generate_embeddings_with_server(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        println!(
            "🚀 Starting embedding generation workflow for model '{}'",
            self.config.model
        );

        let process = match self.start_server().await {
            Ok(Some(p)) => Some(p),
            Ok(None) => {
                println!("ℹ️  Using existing Ollama server instance");
                None
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to start Ollama server: {}. Make sure 'ollama' is installed and in PATH.",
                    e
                ));
            }
        };

        // Ensure model is available (will pull if needed)
        if let Err(e) = self.ensure_model_available().await {
            self.stop_server(process).await;
            return Err(anyhow::anyhow!(
                "Failed to ensure model '{}' is available: {}",
                self.config.model,
                e
            ));
        }

        // Generate embeddings
        let result = self.generate_embeddings(texts).await;

        // Always stop the server if we started it, even if embedding generation failed
        self.stop_server(process).await;

        match &result {
            Ok(_) => println!("✅ Embedding generation workflow completed successfully"),
            Err(e) => println!("Embedding generation workflow failed: {}", e),
        }

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
    }
}
