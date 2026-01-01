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
