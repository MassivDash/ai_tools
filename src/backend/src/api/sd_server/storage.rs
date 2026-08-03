use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct SDImageMetadata {
    pub filename: String,
    pub prompt: String,
    pub diffusion_model: String,
    pub width: i64,
    pub height: i64,
    pub steps: Option<i64>,
    pub cfg_scale: f32,
    pub seed: Option<i64>,
    pub created_at: i64,
    pub additional_info: Option<String>, // JSON string
}

pub struct SDImagesStorage {
    pool: Pool<Sqlite>,
}

impl SDImagesStorage {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn init(&self) -> Result<(), sqlx::Error> {
        // Initialize table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sd_images (
                filename TEXT PRIMARY KEY,
                prompt TEXT NOT NULL,
                diffusion_model TEXT NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                steps INTEGER,
                cfg_scale REAL NOT NULL,
                seed INTEGER,
                created_at INTEGER NOT NULL,
                additional_info TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn add_image(&self, image: SDImageMetadata) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO sd_images (
                filename, prompt, diffusion_model, width, height, steps, cfg_scale, seed, created_at, additional_info
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&image.filename)
        .bind(&image.prompt)
        .bind(&image.diffusion_model)
        .bind(image.width)
        .bind(image.height)
        .bind(image.steps)
        .bind(image.cfg_scale)
        .bind(image.seed)
        .bind(image.created_at)
        .bind(&image.additional_info)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_images(&self) -> Result<Vec<SDImageMetadata>, sqlx::Error> {
        sqlx::query_as::<_, SDImageMetadata>(
            r#"
            SELECT * FROM sd_images ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn delete_image(&self, filename: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM sd_images WHERE filename = ?")
            .bind(filename)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

/// Build an initialized `SDImagesStorage` on a private in-memory SQLite database.
///
/// `:memory:` databases are per-connection, so the pool is pinned to a single
/// connection - otherwise some queries would see an empty database.
#[cfg(test)]
pub(crate) async fn new_test_storage() -> SDImagesStorage {
    use sqlx::sqlite::SqlitePoolOptions;

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");

    let storage = SDImagesStorage::new(pool);
    storage.init().await.expect("Failed to initialize storage");
    storage
}

/// Build an initialized `SDImagesStorage` on a throwaway on-disk SQLite file.
///
/// Needed wherever the storage is used from more than one runtime: a `:memory:`
/// database lives inside a single connection, and the pool reconnecting on
/// another runtime would land on a fresh, empty database. The returned `TempDir`
/// must be kept alive for as long as the storage is used.
#[cfg(test)]
pub(crate) async fn new_file_test_storage() -> (tempfile::TempDir, SDImagesStorage) {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let options = SqliteConnectOptions::new()
        .filename(dir.path().join("sd_images.db"))
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .expect("Failed to connect to temp database");

    let storage = SDImagesStorage::new(pool);
    storage.init().await.expect("Failed to initialize storage");
    (dir, storage)
}

/// Like `new_test_storage`, but `init()` is never called, so the `sd_images`
/// table does not exist and every query fails. Used by the handler tests to
/// exercise the arms that map storage errors onto `500` responses.
#[cfg(test)]
pub(crate) async fn new_broken_test_storage() -> SDImagesStorage {
    use sqlx::sqlite::SqlitePoolOptions;

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");

    SDImagesStorage::new(pool)
}

/// A metadata row with predictable values, for tests.
#[cfg(test)]
pub(crate) fn test_image(filename: &str, created_at: i64) -> SDImageMetadata {
    SDImageMetadata {
        filename: filename.to_string(),
        prompt: "a cat".to_string(),
        diffusion_model: "model.gguf".to_string(),
        width: 512,
        height: 768,
        steps: Some(20),
        cfg_scale: 1.5,
        seed: Some(42),
        created_at,
        additional_info: Some(r#"{"sampler":"euler"}"#.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_is_idempotent_and_starts_empty() {
        let storage = new_test_storage().await;
        // Running init twice must not fail or wipe data.
        storage.add_image(test_image("a.png", 1)).await.unwrap();
        storage.init().await.unwrap();

        let images = storage.get_images().await.unwrap();
        assert_eq!(images.len(), 1);
    }

    #[tokio::test]
    async fn test_add_and_get_image_round_trips_every_field() {
        let storage = new_test_storage().await;
        storage
            .add_image(test_image("cat.png", 1234))
            .await
            .unwrap();

        let images = storage.get_images().await.unwrap();
        assert_eq!(images.len(), 1);
        let img = &images[0];
        assert_eq!(img.filename, "cat.png");
        assert_eq!(img.prompt, "a cat");
        assert_eq!(img.diffusion_model, "model.gguf");
        assert_eq!(img.width, 512);
        assert_eq!(img.height, 768);
        assert_eq!(img.steps, Some(20));
        assert_eq!(img.cfg_scale, 1.5);
        assert_eq!(img.seed, Some(42));
        assert_eq!(img.created_at, 1234);
        assert_eq!(
            img.additional_info.as_deref(),
            Some(r#"{"sampler":"euler"}"#)
        );
    }

    #[tokio::test]
    async fn test_nullable_columns_round_trip_as_none() {
        let storage = new_test_storage().await;
        let mut image = test_image("sparse.png", 1);
        image.steps = None;
        image.seed = None;
        image.additional_info = None;
        storage.add_image(image).await.unwrap();

        let img = &storage.get_images().await.unwrap()[0];
        assert!(img.steps.is_none());
        assert!(img.seed.is_none());
        assert!(img.additional_info.is_none());
    }

    #[tokio::test]
    async fn test_get_images_returns_newest_first() {
        let storage = new_test_storage().await;
        storage.add_image(test_image("old.png", 100)).await.unwrap();
        storage.add_image(test_image("new.png", 300)).await.unwrap();
        storage.add_image(test_image("mid.png", 200)).await.unwrap();

        let names: Vec<String> = storage
            .get_images()
            .await
            .unwrap()
            .into_iter()
            .map(|i| i.filename)
            .collect();
        assert_eq!(names, vec!["new.png", "mid.png", "old.png"]);
    }

    #[tokio::test]
    async fn test_duplicate_filename_is_rejected_by_primary_key() {
        let storage = new_test_storage().await;
        storage.add_image(test_image("dup.png", 1)).await.unwrap();

        let result = storage.add_image(test_image("dup.png", 2)).await;
        assert!(result.is_err());
        assert_eq!(storage.get_images().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_delete_image_removes_only_the_named_row() {
        let storage = new_test_storage().await;
        storage.add_image(test_image("keep.png", 1)).await.unwrap();
        storage.add_image(test_image("drop.png", 2)).await.unwrap();

        storage.delete_image("drop.png").await.unwrap();

        let names: Vec<String> = storage
            .get_images()
            .await
            .unwrap()
            .into_iter()
            .map(|i| i.filename)
            .collect();
        assert_eq!(names, vec!["keep.png"]);
    }

    #[tokio::test]
    async fn test_delete_missing_image_is_not_an_error() {
        let storage = new_test_storage().await;
        // Deleting a row that was never there succeeds silently - the sd log
        // reader relies on this when cleaning up images that never landed.
        assert!(storage.delete_image("never-existed.png").await.is_ok());
    }
}
