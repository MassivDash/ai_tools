use crate::api::default_configs::types::{ChromaDBDefaultConfig, LlamaDefaultConfig};
use anyhow::{Context, Result};
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};
use std::path::Path;

/// SQLite-based storage for default configs
pub struct DefaultConfigsStorage {
    pool: SqlitePool,
}

impl DefaultConfigsStorage {
    /// Create a new default configs storage
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref();

        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create database directory")?;
        }

        // Get absolute path
        let absolute_path = if db_path.exists() {
            db_path
                .canonicalize()
                .context("Failed to canonicalize existing database path")?
        } else {
            let parent = db_path.parent().unwrap_or(Path::new("."));
            let parent_abs = parent
                .canonicalize()
                .or_else(|_| std::env::current_dir().map(|d| d.join(parent)))
                .context("Failed to get absolute path for database directory")?;
            let filename = db_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("conversations.db");
            parent_abs.join(filename)
        };

        println!(
            "💾 Connecting to SQLite database for default configs at: {}",
            absolute_path.display()
        );

        let options = SqliteConnectOptions::new()
            .filename(&absolute_path)
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await.context(format!(
            "Failed to connect to SQLite database at: {}",
            absolute_path.display()
        ))?;

        // Create default_configs table
        println!("📋 Creating default_configs table if it doesn't exist...");
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS default_configs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                config_type TEXT NOT NULL UNIQUE,
                hf_model TEXT,
                embedding_model TEXT,
                updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            )",
        )
        .execute(&pool)
        .await
        .context("Failed to create default_configs table")?;
        println!("✅ default_configs table created/verified");

        Ok(Self { pool })
    }

    /// Get llama default config
    pub async fn get_llama_default(&self) -> Result<Option<LlamaDefaultConfig>> {
        let row = sqlx::query("SELECT hf_model FROM default_configs WHERE config_type = 'llama'")
            .fetch_optional(&self.pool)
            .await
            .context("Failed to fetch llama default config")?;

        if let Some(row) = row {
            let hf_model: Option<String> = row.get(0);
            if let Some(hf_model) = hf_model {
                return Ok(Some(LlamaDefaultConfig { hf_model }));
            }
        }
        Ok(None)
    }

    /// Set llama default config
    pub async fn set_llama_default(&self, config: &LlamaDefaultConfig) -> Result<()> {
        sqlx::query(
            "INSERT INTO default_configs (config_type, hf_model, updated_at)
             VALUES ('llama', ?1, strftime('%s', 'now'))
             ON CONFLICT(config_type) DO UPDATE SET
                 hf_model = ?1,
                 updated_at = strftime('%s', 'now')",
        )
        .bind(&config.hf_model)
        .execute(&self.pool)
        .await
        .context("Failed to set llama default config")?;
        Ok(())
    }

    /// Get chromadb default config
    pub async fn get_chromadb_default(&self) -> Result<Option<ChromaDBDefaultConfig>> {
        let row = sqlx::query(
            "SELECT embedding_model FROM default_configs WHERE config_type = 'chromadb'",
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch chromadb default config")?;

        if let Some(row) = row {
            let embedding_model: Option<String> = row.get(0);
            if let Some(embedding_model) = embedding_model {
                return Ok(Some(ChromaDBDefaultConfig { embedding_model }));
            }
        }
        Ok(None)
    }

    /// Set chromadb default config
    pub async fn set_chromadb_default(&self, config: &ChromaDBDefaultConfig) -> Result<()> {
        sqlx::query(
            "INSERT INTO default_configs (config_type, embedding_model, updated_at)
             VALUES ('chromadb', ?1, strftime('%s', 'now'))
             ON CONFLICT(config_type) DO UPDATE SET
                 embedding_model = ?1,
                 updated_at = strftime('%s', 'now')",
        )
        .bind(&config.embedding_model)
        .execute(&self.pool)
        .await
        .context("Failed to set chromadb default config")?;
        Ok(())
    }
}
