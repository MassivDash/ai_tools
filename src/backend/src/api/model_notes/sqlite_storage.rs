use crate::api::model_notes::types::ModelNote;
use anyhow::{Context, Result};
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};
use std::path::Path;

/// SQLite-based storage for model notes
pub struct ModelNotesStorage {
    pool: SqlitePool,
}

impl ModelNotesStorage {
    /// Create a new model notes storage
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
            "💾 Connecting to SQLite database for model notes at: {}",
            absolute_path.display()
        );

        let options = SqliteConnectOptions::new()
            .filename(&absolute_path)
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await.context(format!(
            "Failed to connect to SQLite database at: {}",
            absolute_path.display()
        ))?;

        // Create model_notes table
        println!("📋 Creating model_notes table if it doesn't exist...");
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS model_notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                platform TEXT NOT NULL,
                model_name TEXT NOT NULL,
                model_path TEXT,
                is_favorite INTEGER NOT NULL DEFAULT 0,
                is_default INTEGER NOT NULL DEFAULT 0,
                tags TEXT,
                notes TEXT,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                UNIQUE(platform, model_name)
            )",
        )
        .execute(&pool)
        .await
        .context("Failed to create model_notes table")?;
        println!("✅ model_notes table created/verified");

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_model_notes_platform_name ON model_notes(platform, model_name)",
        )
        .execute(&pool)
        .await
        .context("Failed to create index")?;
        println!("✅ Index idx_model_notes_platform_name created/verified");

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_model_notes_favorite ON model_notes(is_favorite)",
        )
        .execute(&pool)
        .await
        .context("Failed to create favorite index")?;
        println!("✅ Index idx_model_notes_favorite created/verified");

        // Verify table exists
        let table_exists: Option<i64> = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='model_notes'",
        )
        .fetch_optional(&pool)
        .await
        .context("Failed to verify table existence")?;

        if table_exists.unwrap_or(0) == 0 {
            return Err(anyhow::anyhow!(
                "model_notes table was not created successfully"
            ));
        }
        println!("✅ Verified model_notes table exists");

        Ok(Self { pool })
    }

    /// Get all model notes
    pub async fn get_all_notes(&self) -> Result<Vec<ModelNote>> {
        let rows = sqlx::query(
            "SELECT id, platform, model_name, model_path, is_favorite, is_default, tags, notes, created_at, updated_at 
             FROM model_notes 
             ORDER BY is_favorite DESC, updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch model notes")?;

        let mut notes = Vec::new();
        for row in rows {
            let tags_json: Option<String> = row.get(6);
            let tags: Vec<String> = if let Some(json) = tags_json {
                serde_json::from_str(&json).unwrap_or_default()
            } else {
                Vec::new()
            };

            notes.push(ModelNote {
                id: Some(row.get(0)),
                platform: row.get(1),
                model_name: row.get(2),
                model_path: row.get(3),
                is_favorite: row.get::<i64, _>(4) != 0,
                is_default: row.get::<i64, _>(5) != 0,
                tags,
                notes: row.get(7),
                created_at: Some(row.get(8)),
                updated_at: Some(row.get(9)),
            });
        }

        Ok(notes)
    }

    /// Get a specific model note by platform and model name
    pub async fn get_note(&self, platform: &str, model_name: &str) -> Result<Option<ModelNote>> {
        let row = sqlx::query(
            "SELECT id, platform, model_name, model_path, is_favorite, is_default, tags, notes, created_at, updated_at 
             FROM model_notes 
             WHERE platform = ?1 AND model_name = ?2",
        )
        .bind(platform)
        .bind(model_name)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch model note")?;

        if let Some(row) = row {
            let tags_json: Option<String> = row.get(6);
            let tags: Vec<String> = if let Some(json) = tags_json {
                serde_json::from_str(&json).unwrap_or_default()
            } else {
                Vec::new()
            };

            Ok(Some(ModelNote {
                id: Some(row.get(0)),
                platform: row.get(1),
                model_name: row.get(2),
                model_path: row.get(3),
                is_favorite: row.get::<i64, _>(4) != 0,
                is_default: row.get::<i64, _>(5) != 0,
                tags,
                notes: row.get(7),
                created_at: Some(row.get(8)),
                updated_at: Some(row.get(9)),
            }))
        } else {
            Ok(None)
        }
    }

    /// Create or update a model note
    /// Uses a transaction to ensure atomicity when setting default models
    pub async fn upsert_note(&self, note: &ModelNote) -> Result<ModelNote> {
        let tags_json = serde_json::to_string(&note.tags).context("Failed to serialize tags")?;

        let is_favorite_int = if note.is_favorite { 1 } else { 0 };
        let is_default_int = if note.is_default { 1 } else { 0 };

        println!(
            "🔍 Upserting note: platform={}, model={}, favorite={}, default={}, tags={}, notes={:?}, path={:?}",
            note.platform, note.model_name, is_favorite_int, is_default_int, tags_json, note.notes, note.model_path
        );

        // Start transaction to ensure atomicity
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to start transaction")?;

        // Step 1: If setting this model as default, unset all other defaults for the same platform
        if note.is_default {
            sqlx::query(
                "UPDATE model_notes 
                 SET is_default = 0 
                 WHERE platform = ?1 AND model_name != ?2",
            )
            .bind(&note.platform)
            .bind(&note.model_name)
            .execute(&mut *tx)
            .await
            .context("Failed to unset other defaults")?;
            println!("✅ Unset other defaults for platform: {}", note.platform);
        }

        // Step 2: Try to update existing note
        // For default models, always clear model_path (store only name)
        // For non-default models, update model_path if provided
        let rows_affected = if note.is_default {
            // Default model: clear model_path, store only name
            sqlx::query(
                "UPDATE model_notes 
                 SET is_favorite = ?3, is_default = ?4, tags = ?5, notes = ?6, model_path = NULL, updated_at = strftime('%s', 'now')
                 WHERE platform = ?1 AND model_name = ?2",
            )
            .bind(&note.platform)
            .bind(&note.model_name)
            .bind(is_favorite_int)
            .bind(is_default_int)
            .bind(&tags_json)
            .bind(&note.notes)
            .execute(&mut *tx)
            .await
            .context(format!(
                "Failed to update model note for {}:{}",
                note.platform, note.model_name
            ))?
            .rows_affected()
        } else if note.model_path.is_some() {
            // Non-default model with path: update path
            sqlx::query(
                "UPDATE model_notes 
                 SET is_favorite = ?3, is_default = ?4, tags = ?5, notes = ?6, model_path = ?7, updated_at = strftime('%s', 'now')
                 WHERE platform = ?1 AND model_name = ?2",
            )
            .bind(&note.platform)
            .bind(&note.model_name)
            .bind(is_favorite_int)
            .bind(is_default_int)
            .bind(&tags_json)
            .bind(&note.notes)
            .bind(&note.model_path)
            .execute(&mut *tx)
            .await
            .context(format!(
                "Failed to update model note for {}:{}",
                note.platform, note.model_name
            ))?
            .rows_affected()
        } else {
            // Non-default model without path: don't update path
            sqlx::query(
                "UPDATE model_notes 
                 SET is_favorite = ?3, is_default = ?4, tags = ?5, notes = ?6, updated_at = strftime('%s', 'now')
                 WHERE platform = ?1 AND model_name = ?2",
            )
            .bind(&note.platform)
            .bind(&note.model_name)
            .bind(is_favorite_int)
            .bind(is_default_int)
            .bind(&tags_json)
            .bind(&note.notes)
            .execute(&mut *tx)
            .await
            .context(format!(
                "Failed to update model note for {}:{}",
                note.platform, note.model_name
            ))?
            .rows_affected()
        };

        println!("📊 Update affected {} rows", rows_affected);

        // Step 3: If no rows were updated, insert new note
        if rows_affected == 0 {
            println!(
                "➕ Inserting new note for {}:{}",
                note.platform, note.model_name
            );
            // For default models, don't store model_path (NULL)
            let model_path_for_insert = if note.is_default {
                None
            } else {
                note.model_path.as_ref()
            };
            sqlx::query(
                "INSERT INTO model_notes (platform, model_name, model_path, is_favorite, is_default, tags, notes) 
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .bind(&note.platform)
            .bind(&note.model_name)
            .bind(model_path_for_insert)
            .bind(is_favorite_int)
            .bind(is_default_int)
            .bind(&tags_json)
            .bind(&note.notes)
            .execute(&mut *tx)
            .await
            .context(format!(
                "Failed to insert model note for {}:{}",
                note.platform, note.model_name
            ))?;
            println!("✅ Inserted new note");
        } else {
            println!("✅ Updated existing note");
        }

        // Commit transaction
        tx.commit().await.context("Failed to commit transaction")?;
        println!("✅ Transaction committed");

        // Fetch the updated/inserted note
        match self.get_note(&note.platform, &note.model_name).await {
            Ok(Some(saved_note)) => {
                println!("✅ Retrieved saved note");
                Ok(saved_note)
            }
            Ok(None) => {
                // Try once more after a short delay
                use tokio::time::{sleep, Duration};
                sleep(Duration::from_millis(50)).await;
                self.get_note(&note.platform, &note.model_name)
                    .await
                    .and_then(|opt| {
                        opt.context(format!(
                            "Failed to retrieve note after insert/update for {}:{}",
                            note.platform, note.model_name
                        ))
                    })
            }
            Err(e) => Err(e),
        }
    }

    /// Delete a model note
    pub async fn delete_note(&self, platform: &str, model_name: &str) -> Result<bool> {
        let rows_affected =
            sqlx::query("DELETE FROM model_notes WHERE platform = ?1 AND model_name = ?2")
                .bind(platform)
                .bind(model_name)
                .execute(&self.pool)
                .await
                .context("Failed to delete model note")?
                .rows_affected();

        Ok(rows_affected > 0)
    }

    /// Get the default model for a platform
    pub async fn get_default_model(&self, platform: &str) -> Result<Option<ModelNote>> {
        let row = sqlx::query(
            "SELECT id, platform, model_name, model_path, is_favorite, is_default, tags, notes, created_at, updated_at 
             FROM model_notes 
             WHERE platform = ?1 AND is_default = 1
             LIMIT 1",
        )
        .bind(platform)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch default model")?;

        if let Some(row) = row {
            let tags_json: Option<String> = row.get(6);
            let tags: Vec<String> = if let Some(json) = tags_json {
                serde_json::from_str(&json).unwrap_or_default()
            } else {
                Vec::new()
            };

            Ok(Some(ModelNote {
                id: Some(row.get(0)),
                platform: row.get(1),
                model_name: row.get(2),
                model_path: row.get(3),
                is_favorite: row.get::<i64, _>(4) != 0,
                is_default: row.get::<i64, _>(5) != 0,
                tags,
                notes: row.get(7),
                created_at: Some(row.get(8)),
                updated_at: Some(row.get(9)),
            }))
        } else {
            Ok(None)
        }
    }
}
