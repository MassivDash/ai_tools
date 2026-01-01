use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Sqlite};

#[derive(Clone, Debug, Serialize, Deserialize, FromRow)]
pub struct SDModelSet {
    pub id: i64,
    pub name: String,
    pub diffusion_model: String,
    pub vae: Option<String>,
    pub llm: Option<String>,
    pub is_default: bool,
}

#[derive(Clone)]
pub struct SDModelSetsStorage {
    pool: Pool<Sqlite>,
}

impl SDModelSetsStorage {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn init(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sd_model_sets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                diffusion_model TEXT NOT NULL,
                vae TEXT,
                llm TEXT,
                is_default BOOLEAN NOT NULL DEFAULT 0
            )",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<SDModelSet>, sqlx::Error> {
        sqlx::query_as::<_, SDModelSet>(
            "SELECT id, name, diffusion_model, vae, llm, is_default FROM sd_model_sets ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn create(
        &self,
        name: String,
        diffusion_model: String,
        vae: Option<String>,
        llm: Option<String>,
        is_default: bool,
    ) -> Result<SDModelSet, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        if is_default {
            sqlx::query("UPDATE sd_model_sets SET is_default = 0")
                .execute(&mut *tx)
                .await?;
        }

        let id = sqlx::query(
            "INSERT INTO sd_model_sets (name, diffusion_model, vae, llm, is_default) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&name)
        .bind(&diffusion_model)
        .bind(&vae)
        .bind(&llm)
        .bind(is_default)
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();

        tx.commit().await?;

        Ok(SDModelSet {
            id,
            name,
            diffusion_model,
            vae,
            llm,
            is_default,
        })
    }

    pub async fn update(
        &self,
        id: i64,
        name: String,
        diffusion_model: String,
        vae: Option<String>,
        llm: Option<String>,
        is_default: bool,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        if is_default {
            sqlx::query("UPDATE sd_model_sets SET is_default = 0")
                .execute(&mut *tx)
                .await?;
        }

        sqlx::query(
            "UPDATE sd_model_sets SET name = ?, diffusion_model = ?, vae = ?, llm = ?, is_default = ? WHERE id = ?",
        )
        .bind(name)
        .bind(diffusion_model)
        .bind(vae)
        .bind(llm)
        .bind(is_default)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM sd_model_sets WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_default(&self) -> Result<Option<SDModelSet>, sqlx::Error> {
        sqlx::query_as::<_, SDModelSet>(
            "SELECT id, name, diffusion_model, vae, llm, is_default FROM sd_model_sets WHERE is_default = 1 LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
    }
}
