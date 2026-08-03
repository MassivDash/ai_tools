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

    /// Drop the backing table so that every subsequent query fails.
    ///
    /// Used by the handler tests to exercise the `Err(..)` arms that map storage
    /// failures onto `500` responses; there is no other way to make a healthy
    /// in-memory/temp-file database fail on demand.
    #[cfg(test)]
    pub(crate) async fn drop_table_for_tests(&self) {
        sqlx::query("DROP TABLE model_notes")
            .execute(&self.pool)
            .await
            .expect("Failed to drop model_notes table");
    }
}

/// Build a `ModelNotesStorage` backed by a throwaway on-disk SQLite file.
///
/// `ModelNotesStorage::new` takes a filesystem path (it canonicalizes it and
/// creates the parent directory), so a `sqlite::memory:` URL is not an option
/// here the way it is for the pool-based storages. The returned `TempDir` must
/// be kept alive for as long as the storage is used.
#[cfg(test)]
pub(crate) async fn new_test_storage() -> (tempfile::TempDir, ModelNotesStorage) {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let storage = ModelNotesStorage::new(dir.path().join("model_notes.db"))
        .await
        .expect("Failed to initialize storage");
    (dir, storage)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(platform: &str, model_name: &str) -> ModelNote {
        ModelNote {
            id: None,
            platform: platform.to_string(),
            model_name: model_name.to_string(),
            model_path: None,
            is_favorite: false,
            is_default: false,
            tags: Vec::new(),
            notes: None,
            created_at: None,
            updated_at: None,
        }
    }

    #[tokio::test]
    async fn test_new_creates_empty_table() {
        let (_dir, storage) = new_test_storage().await;
        assert!(storage.get_all_notes().await.unwrap().is_empty());
        assert!(storage
            .get_note("llama", "missing")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_new_is_idempotent_and_keeps_existing_rows() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("nested").join("model_notes.db");

        let storage = ModelNotesStorage::new(&db).await.unwrap();
        storage.upsert_note(&note("llama", "kept")).await.unwrap();
        drop(storage);

        // Re-opening an existing file must not wipe the table.
        let reopened = ModelNotesStorage::new(&db).await.unwrap();
        let notes = reopened.get_all_notes().await.unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].model_name, "kept");
    }

    #[tokio::test]
    async fn test_upsert_inserts_new_note_with_timestamps_and_id() {
        let (_dir, storage) = new_test_storage().await;

        let mut incoming = note("llama", "qwen3-4b");
        incoming.model_path = Some("/models/qwen3-4b.gguf".to_string());
        incoming.is_favorite = true;
        incoming.tags = vec!["fast".to_string(), "small".to_string()];
        incoming.notes = Some("works well".to_string());

        let saved = storage.upsert_note(&incoming).await.unwrap();

        assert!(saved.id.is_some());
        assert!(saved.created_at.is_some());
        assert!(saved.updated_at.is_some());
        assert_eq!(saved.model_path, Some("/models/qwen3-4b.gguf".to_string()));
        assert!(saved.is_favorite);
        assert!(!saved.is_default);
        assert_eq!(saved.tags, vec!["fast", "small"]);
        assert_eq!(saved.notes.as_deref(), Some("works well"));
    }

    #[tokio::test]
    async fn test_upsert_updates_instead_of_duplicating() {
        let (_dir, storage) = new_test_storage().await;

        let first = storage.upsert_note(&note("llama", "same")).await.unwrap();

        let mut second = note("llama", "same");
        second.is_favorite = true;
        second.notes = Some("updated".to_string());
        let updated = storage.upsert_note(&second).await.unwrap();

        // Same row (UNIQUE(platform, model_name)), not a second insert.
        assert_eq!(updated.id, first.id);
        assert!(updated.is_favorite);
        assert_eq!(updated.notes.as_deref(), Some("updated"));
        assert_eq!(storage.get_all_notes().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_upsert_without_path_preserves_existing_path() {
        let (_dir, storage) = new_test_storage().await;

        let mut with_path = note("llama", "keep-path");
        with_path.model_path = Some("/models/keep.gguf".to_string());
        storage.upsert_note(&with_path).await.unwrap();

        // A non-default upsert that carries no path must not clear the stored one.
        let mut without_path = note("llama", "keep-path");
        without_path.is_favorite = true;
        let updated = storage.upsert_note(&without_path).await.unwrap();

        assert_eq!(updated.model_path, Some("/models/keep.gguf".to_string()));
        assert!(updated.is_favorite);
    }

    #[tokio::test]
    async fn test_upsert_with_new_path_replaces_existing_path() {
        let (_dir, storage) = new_test_storage().await;

        let mut first = note("llama", "swap-path");
        first.model_path = Some("/models/old.gguf".to_string());
        storage.upsert_note(&first).await.unwrap();

        let mut second = note("llama", "swap-path");
        second.model_path = Some("/models/new.gguf".to_string());
        let updated = storage.upsert_note(&second).await.unwrap();

        assert_eq!(updated.model_path, Some("/models/new.gguf".to_string()));
    }

    #[tokio::test]
    async fn test_default_note_never_stores_a_path() {
        let (_dir, storage) = new_test_storage().await;

        // On insert.
        let mut inserted = note("llama", "default-model");
        inserted.is_default = true;
        inserted.model_path = Some("/models/ignored.gguf".to_string());
        let saved = storage.upsert_note(&inserted).await.unwrap();
        assert!(saved.is_default);
        assert!(saved.model_path.is_none());

        // And on update of a row that previously had a path.
        let mut with_path = note("ollama", "switcher");
        with_path.model_path = Some("/models/present.gguf".to_string());
        storage.upsert_note(&with_path).await.unwrap();

        let mut promote = note("ollama", "switcher");
        promote.is_default = true;
        promote.model_path = Some("/models/still-ignored.gguf".to_string());
        let promoted = storage.upsert_note(&promote).await.unwrap();
        assert!(promoted.is_default);
        assert!(promoted.model_path.is_none());
    }

    #[tokio::test]
    async fn test_setting_default_unsets_other_defaults_on_same_platform_only() {
        let (_dir, storage) = new_test_storage().await;

        let mut llama_a = note("llama", "a");
        llama_a.is_default = true;
        storage.upsert_note(&llama_a).await.unwrap();

        let mut ollama_x = note("ollama", "x");
        ollama_x.is_default = true;
        storage.upsert_note(&ollama_x).await.unwrap();

        let mut llama_b = note("llama", "b");
        llama_b.is_default = true;
        storage.upsert_note(&llama_b).await.unwrap();

        // Only one default per platform, and the other platform is untouched.
        assert_eq!(
            storage
                .get_default_model("llama")
                .await
                .unwrap()
                .unwrap()
                .model_name,
            "b"
        );
        assert!(
            !storage
                .get_note("llama", "a")
                .await
                .unwrap()
                .unwrap()
                .is_default
        );
        assert_eq!(
            storage
                .get_default_model("ollama")
                .await
                .unwrap()
                .unwrap()
                .model_name,
            "x"
        );
    }

    #[tokio::test]
    async fn test_get_default_model_none_when_nothing_is_default() {
        let (_dir, storage) = new_test_storage().await;
        storage.upsert_note(&note("llama", "plain")).await.unwrap();

        assert!(storage.get_default_model("llama").await.unwrap().is_none());
        assert!(storage.get_default_model("ollama").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_favorites_are_listed_first() {
        let (_dir, storage) = new_test_storage().await;

        storage
            .upsert_note(&note("llama", "plain-1"))
            .await
            .unwrap();
        storage
            .upsert_note(&note("llama", "plain-2"))
            .await
            .unwrap();

        let mut favorite = note("llama", "starred");
        favorite.is_favorite = true;
        storage.upsert_note(&favorite).await.unwrap();

        let notes = storage.get_all_notes().await.unwrap();
        assert_eq!(notes.len(), 3);
        assert_eq!(notes[0].model_name, "starred");
        assert!(notes[1..].iter().all(|n| !n.is_favorite));
    }

    #[tokio::test]
    async fn test_tags_round_trip_including_empty_and_unicode() {
        let (_dir, storage) = new_test_storage().await;

        let mut empty_tags = note("llama", "no-tags");
        empty_tags.tags = Vec::new();
        let saved = storage.upsert_note(&empty_tags).await.unwrap();
        assert!(saved.tags.is_empty());

        let mut unicode_tags = note("ollama", "tagged");
        unicode_tags.tags = vec!["schnell ⚡".to_string(), "日本語".to_string()];
        let saved = storage.upsert_note(&unicode_tags).await.unwrap();
        assert_eq!(saved.tags, vec!["schnell ⚡", "日本語"]);

        // And after a fresh read from the DB.
        let reread = storage.get_note("ollama", "tagged").await.unwrap().unwrap();
        assert_eq!(reread.tags, vec!["schnell ⚡", "日本語"]);
    }

    #[tokio::test]
    async fn test_same_model_name_on_two_platforms_are_separate_rows() {
        let (_dir, storage) = new_test_storage().await;

        storage.upsert_note(&note("llama", "shared")).await.unwrap();
        storage
            .upsert_note(&note("ollama", "shared"))
            .await
            .unwrap();

        assert_eq!(storage.get_all_notes().await.unwrap().len(), 2);
        assert!(storage.get_note("llama", "shared").await.unwrap().is_some());
        assert!(storage
            .get_note("ollama", "shared")
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn test_delete_note() {
        let (_dir, storage) = new_test_storage().await;
        storage.upsert_note(&note("llama", "doomed")).await.unwrap();

        assert!(storage.delete_note("llama", "doomed").await.unwrap());
        assert!(storage.get_note("llama", "doomed").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_note_returns_false_when_missing() {
        let (_dir, storage) = new_test_storage().await;
        storage
            .upsert_note(&note("llama", "present"))
            .await
            .unwrap();

        // Wrong name, and right name on the wrong platform.
        assert!(!storage.delete_note("llama", "absent").await.unwrap());
        assert!(!storage.delete_note("ollama", "present").await.unwrap());
        assert_eq!(storage.get_all_notes().await.unwrap().len(), 1);
    }
}
