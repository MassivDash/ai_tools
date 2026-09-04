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
        let db_path_str = db_path.to_str().unwrap_or("");

        // Handle in-memory database specially (skip file system operations)
        let (db_path_for_connection, display_path) = if db_path_str == ":memory:" {
            println!("💾 Connecting to SQLite in-memory database for default configs");
            (":memory:".to_string(), ":memory:".to_string())
        } else {
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

            let display = absolute_path.display().to_string();
            println!(
                "💾 Connecting to SQLite database for default configs at: {}",
                display
            );
            (absolute_path.to_str().unwrap().to_string(), display)
        };

        let options = SqliteConnectOptions::new()
            .filename(&db_path_for_connection)
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await.context(format!(
            "Failed to connect to SQLite database at: {}",
            display_path
        ))?;

        // Create default_configs table
        println!("📋 Creating default_configs table if it doesn't exist...");
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS default_configs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                config_type TEXT NOT NULL UNIQUE,
                hf_model TEXT,
                embedding_model TEXT,
                ctx_size INTEGER DEFAULT 0,
                threads INTEGER,
                threads_batch INTEGER,
                predict INTEGER,
                batch_size INTEGER,
                ubatch_size INTEGER,
                flash_attn BOOLEAN,
                mlock BOOLEAN,
                no_mmap BOOLEAN,
                gpu_layers INTEGER,
                n_cpu_moe INTEGER,
                model TEXT,
                updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            )",
        )
        .execute(&pool)
        .await
        .context("Failed to create default_configs table")?;

        let alter_queries = [
            "ALTER TABLE default_configs ADD COLUMN ctx_size INTEGER DEFAULT 0",
            "ALTER TABLE default_configs ADD COLUMN threads INTEGER",
            "ALTER TABLE default_configs ADD COLUMN threads_batch INTEGER",
            "ALTER TABLE default_configs ADD COLUMN predict INTEGER",
            "ALTER TABLE default_configs ADD COLUMN batch_size INTEGER",
            "ALTER TABLE default_configs ADD COLUMN ubatch_size INTEGER",
            "ALTER TABLE default_configs ADD COLUMN flash_attn BOOLEAN",
            "ALTER TABLE default_configs ADD COLUMN mlock BOOLEAN",
            "ALTER TABLE default_configs ADD COLUMN no_mmap BOOLEAN",
            "ALTER TABLE default_configs ADD COLUMN gpu_layers INTEGER",
            "ALTER TABLE default_configs ADD COLUMN n_cpu_moe INTEGER",
            "ALTER TABLE default_configs ADD COLUMN model TEXT",
        ];

        for query in alter_queries {
            let _ = sqlx::query(query).execute(&pool).await;
        }

        println!("✅ default_configs table created/verified");

        Ok(Self { pool })
    }

    /// Get llama default config
    pub async fn get_llama_default(&self) -> Result<Option<LlamaDefaultConfig>> {
        let row = sqlx::query(
            "SELECT hf_model, ctx_size, threads, threads_batch, predict, batch_size, ubatch_size, flash_attn, mlock, no_mmap, gpu_layers, n_cpu_moe, model FROM default_configs WHERE config_type = 'llama'",
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch llama default config")?;

        if let Some(row) = row {
            let hf_model: Option<String> = row.get(0);
            if let Some(hf_model) = hf_model {
                let ctx_size: i64 = row.try_get(1).unwrap_or(0);
                let threads: Option<i32> = row.try_get(2).unwrap_or(None);
                let threads_batch: Option<i32> = row.try_get(3).unwrap_or(None);
                let predict: Option<i32> = row.try_get(4).unwrap_or(None);
                let batch_size: Option<i64> = row.try_get(5).unwrap_or(None);
                let ubatch_size: Option<i64> = row.try_get(6).unwrap_or(None);
                let flash_attn: Option<bool> = row.try_get(7).unwrap_or(None);
                let mlock: Option<bool> = row.try_get(8).unwrap_or(None);
                let no_mmap: Option<bool> = row.try_get(9).unwrap_or(None);
                let gpu_layers: Option<i64> = row.try_get(10).unwrap_or(None);
                let n_cpu_moe: Option<i64> = row.try_get(11).unwrap_or(None);
                let model: Option<String> = row.try_get(12).unwrap_or(None);

                return Ok(Some(LlamaDefaultConfig {
                    hf_model,
                    ctx_size: ctx_size as u32,
                    threads,
                    threads_batch,
                    predict,
                    batch_size: batch_size.map(|v| v as u32),
                    ubatch_size: ubatch_size.map(|v| v as u32),
                    flash_attn,
                    mlock,
                    no_mmap,
                    gpu_layers: gpu_layers.map(|v| v as u32),
                    n_cpu_moe: n_cpu_moe.map(|v| v as u32),
                    model,
                }));
            }
        }
        Ok(None)
    }

    /// Set llama default config
    pub async fn set_llama_default(&self, config: &LlamaDefaultConfig) -> Result<()> {
        sqlx::query(
            "INSERT INTO default_configs (config_type, hf_model, ctx_size, threads, threads_batch, predict, batch_size, ubatch_size, flash_attn, mlock, no_mmap, gpu_layers, n_cpu_moe, model, updated_at)
             VALUES ('llama', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, strftime('%s', 'now'))
             ON CONFLICT(config_type) DO UPDATE SET
                 hf_model = ?1,
                 ctx_size = ?2,
                 threads = ?3,
                 threads_batch = ?4,
                 predict = ?5,
                 batch_size = ?6,
                 ubatch_size = ?7,
                 flash_attn = ?8,
                 mlock = ?9,
                 no_mmap = ?10,
                 gpu_layers = ?11,
                 n_cpu_moe = ?12,
                 model = ?13,
                 updated_at = strftime('%s', 'now')",
        )
        .bind(&config.hf_model)
        .bind(config.ctx_size as i64)
        .bind(config.threads)
        .bind(config.threads_batch)
        .bind(config.predict)
        .bind(config.batch_size.map(|v| v as i64))
        .bind(config.ubatch_size.map(|v| v as i64))
        .bind(config.flash_attn)
        .bind(config.mlock)
        .bind(config.no_mmap)
        .bind(config.gpu_layers.map(|v| v as i64))
        .bind(config.n_cpu_moe.map(|v| v as i64))
        .bind(&config.model)
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

#[cfg(test)]
pub mod tests {
    use super::*;

    #[tokio::test]
    async fn test_llama_default_config_save_and_restore() {
        let storage = DefaultConfigsStorage::new(":memory:").await.unwrap();
        let initial = storage.get_llama_default().await.unwrap();
        assert!(initial.is_none());

        let target = LlamaDefaultConfig {
            hf_model: "unsloth/NVIDIA-Nemotron-3.5-Lightning-30B-A3B-GGUF:MXFP4_MOE".to_string(),
            ctx_size: 32768,
            threads: Some(16),
            threads_batch: Some(8),
            predict: Some(512),
            batch_size: Some(1024),
            ubatch_size: Some(256),
            flash_attn: Some(true),
            mlock: Some(false),
            no_mmap: Some(true),
            gpu_layers: Some(99),
            n_cpu_moe: Some(32),
            model: None,
        };

        storage.set_llama_default(&target).await.unwrap();
        let loaded = storage.get_llama_default().await.unwrap().unwrap();
        assert_eq!(loaded, target);
    }
}
