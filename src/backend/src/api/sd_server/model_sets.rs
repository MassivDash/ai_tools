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

/// Build an initialized `SDModelSetsStorage` on a private in-memory SQLite database.
///
/// `:memory:` databases are per-connection, so the pool is pinned to a single
/// connection - otherwise some queries would see an empty database.
#[cfg(test)]
pub(crate) async fn new_test_storage() -> SDModelSetsStorage {
    use sqlx::sqlite::SqlitePoolOptions;

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");

    let storage = SDModelSetsStorage::new(pool);
    storage.init().await.expect("Failed to initialize storage");
    storage
}

/// Like `new_test_storage`, but `init()` is never called, so the `sd_model_sets`
/// table does not exist and every query fails. Used by the handler tests to
/// exercise the arms that map storage errors onto `500` responses.
#[cfg(test)]
pub(crate) async fn new_broken_test_storage() -> SDModelSetsStorage {
    use sqlx::sqlite::SqlitePoolOptions;

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");

    SDModelSetsStorage::new(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_is_empty_before_anything_is_created() {
        let storage = new_test_storage().await;
        assert!(storage.list().await.unwrap().is_empty());
        assert!(storage.get_default().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_create_returns_the_inserted_row() {
        let storage = new_test_storage().await;

        let created = storage
            .create(
                "flux".to_string(),
                "flux1-dev.gguf".to_string(),
                Some("ae.safetensors".to_string()),
                Some("t5.gguf".to_string()),
                false,
            )
            .await
            .unwrap();

        assert!(created.id > 0);
        assert_eq!(created.name, "flux");
        assert_eq!(created.diffusion_model, "flux1-dev.gguf");
        assert_eq!(created.vae.as_deref(), Some("ae.safetensors"));
        assert_eq!(created.llm.as_deref(), Some("t5.gguf"));
        assert!(!created.is_default);

        // And it is really persisted, with the optional columns intact.
        let listed = storage.list().await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, created.id);
        assert_eq!(listed[0].vae.as_deref(), Some("ae.safetensors"));
    }

    #[tokio::test]
    async fn test_create_with_null_optionals() {
        let storage = new_test_storage().await;
        storage
            .create("bare".to_string(), "m.gguf".to_string(), None, None, false)
            .await
            .unwrap();

        let set = &storage.list().await.unwrap()[0];
        assert!(set.vae.is_none());
        assert!(set.llm.is_none());
    }

    #[tokio::test]
    async fn test_list_is_ordered_by_name() {
        let storage = new_test_storage().await;
        for name in ["zebra", "alpha", "mango"] {
            storage
                .create(name.to_string(), "m.gguf".to_string(), None, None, false)
                .await
                .unwrap();
        }

        let names: Vec<String> = storage
            .list()
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(names, vec!["alpha", "mango", "zebra"]);
    }

    #[tokio::test]
    async fn test_creating_a_default_clears_the_previous_default() {
        let storage = new_test_storage().await;

        let first = storage
            .create("first".to_string(), "a.gguf".to_string(), None, None, true)
            .await
            .unwrap();
        assert_eq!(storage.get_default().await.unwrap().unwrap().id, first.id);

        let second = storage
            .create("second".to_string(), "b.gguf".to_string(), None, None, true)
            .await
            .unwrap();

        let default = storage.get_default().await.unwrap().unwrap();
        assert_eq!(default.id, second.id);
        // Exactly one row is flagged as default.
        let defaults = storage
            .list()
            .await
            .unwrap()
            .into_iter()
            .filter(|s| s.is_default)
            .count();
        assert_eq!(defaults, 1);
    }

    #[tokio::test]
    async fn test_creating_a_non_default_leaves_the_existing_default_alone() {
        let storage = new_test_storage().await;
        let kept = storage
            .create("kept".to_string(), "a.gguf".to_string(), None, None, true)
            .await
            .unwrap();
        storage
            .create("other".to_string(), "b.gguf".to_string(), None, None, false)
            .await
            .unwrap();

        assert_eq!(storage.get_default().await.unwrap().unwrap().id, kept.id);
    }

    #[tokio::test]
    async fn test_update_replaces_every_field() {
        let storage = new_test_storage().await;
        let created = storage
            .create(
                "before".to_string(),
                "a.gguf".to_string(),
                Some("old-vae".to_string()),
                Some("old-llm".to_string()),
                false,
            )
            .await
            .unwrap();

        storage
            .update(
                created.id,
                "after".to_string(),
                "b.gguf".to_string(),
                None,
                Some("new-llm".to_string()),
                true,
            )
            .await
            .unwrap();

        let updated = &storage.list().await.unwrap()[0];
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.name, "after");
        assert_eq!(updated.diffusion_model, "b.gguf");
        assert!(updated.vae.is_none());
        assert_eq!(updated.llm.as_deref(), Some("new-llm"));
        assert!(updated.is_default);
    }

    #[tokio::test]
    async fn test_updating_a_row_to_default_clears_the_others() {
        let storage = new_test_storage().await;
        storage
            .create("a".to_string(), "a.gguf".to_string(), None, None, true)
            .await
            .unwrap();
        let b = storage
            .create("b".to_string(), "b.gguf".to_string(), None, None, false)
            .await
            .unwrap();

        storage
            .update(
                b.id,
                "b".to_string(),
                "b.gguf".to_string(),
                None,
                None,
                true,
            )
            .await
            .unwrap();

        assert_eq!(storage.get_default().await.unwrap().unwrap().id, b.id);
        assert_eq!(
            storage
                .list()
                .await
                .unwrap()
                .into_iter()
                .filter(|s| s.is_default)
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn test_update_of_unknown_id_is_a_no_op() {
        let storage = new_test_storage().await;
        storage
            .create("only".to_string(), "a.gguf".to_string(), None, None, false)
            .await
            .unwrap();

        // No row matches, so nothing changes and no error is raised.
        storage
            .update(
                9999,
                "ghost".to_string(),
                "x.gguf".to_string(),
                None,
                None,
                false,
            )
            .await
            .unwrap();

        let sets = storage.list().await.unwrap();
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].name, "only");
    }

    #[tokio::test]
    async fn test_updating_an_unknown_id_to_default_still_clears_defaults() {
        let storage = new_test_storage().await;
        storage
            .create("a".to_string(), "a.gguf".to_string(), None, None, true)
            .await
            .unwrap();

        // The "clear all defaults" step runs before the (no-op) UPDATE, so the
        // caller is left with no default at all.
        storage
            .update(
                9999,
                "ghost".to_string(),
                "x.gguf".to_string(),
                None,
                None,
                true,
            )
            .await
            .unwrap();

        assert!(storage.get_default().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_removes_only_the_given_id() {
        let storage = new_test_storage().await;
        let keep = storage
            .create("keep".to_string(), "a.gguf".to_string(), None, None, false)
            .await
            .unwrap();
        let drop = storage
            .create("drop".to_string(), "b.gguf".to_string(), None, None, false)
            .await
            .unwrap();

        storage.delete(drop.id).await.unwrap();

        let sets = storage.list().await.unwrap();
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].id, keep.id);
    }

    #[tokio::test]
    async fn test_delete_of_unknown_id_is_not_an_error() {
        let storage = new_test_storage().await;
        assert!(storage.delete(4242).await.is_ok());
    }

    #[tokio::test]
    async fn test_duplicate_names_are_allowed() {
        let storage = new_test_storage().await;
        // There is no UNIQUE constraint on name, so two sets can share one.
        storage
            .create("same".to_string(), "a.gguf".to_string(), None, None, false)
            .await
            .unwrap();
        storage
            .create("same".to_string(), "b.gguf".to_string(), None, None, false)
            .await
            .unwrap();

        assert_eq!(storage.list().await.unwrap().len(), 2);
    }
}
