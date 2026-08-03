use crate::api::agent::tool_groups::types::ToolGroup;
use anyhow::{Context, Result};
use sqlx::{Row, SqlitePool};

#[derive(Clone)]
pub struct ToolGroupsStorage {
    pool: SqlitePool,
}

impl ToolGroupsStorage {
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        let storage = Self { pool };
        storage.initialize().await?;
        Ok(storage)
    }

    async fn initialize(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS tool_groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                tool_types TEXT NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            )",
        )
        .execute(&self.pool)
        .await
        .context("Failed to create tool_groups table")?;

        Ok(())
    }

    fn row_to_group(row: &sqlx::sqlite::SqliteRow) -> Result<ToolGroup> {
        let tool_types_json: String = row.get(2);
        let tool_types: Vec<String> =
            serde_json::from_str(&tool_types_json).context("Failed to parse tool_types")?;

        Ok(ToolGroup {
            id: row.get(0),
            name: row.get(1),
            tool_types,
            created_at: row.get(3),
            updated_at: row.get(4),
        })
    }

    pub async fn get_all_groups(&self) -> Result<Vec<ToolGroup>> {
        let rows = sqlx::query(
            "SELECT id, name, tool_types, created_at, updated_at FROM tool_groups ORDER BY name ASC",
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch tool groups")?;

        rows.iter().map(Self::row_to_group).collect()
    }

    pub async fn get_group(&self, id: i64) -> Result<Option<ToolGroup>> {
        let row = sqlx::query(
            "SELECT id, name, tool_types, created_at, updated_at FROM tool_groups WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch tool group")?;

        row.as_ref().map(Self::row_to_group).transpose()
    }

    pub async fn create_group(&self, name: &str, tool_types: &[String]) -> Result<ToolGroup> {
        let tool_types_json =
            serde_json::to_string(tool_types).context("Failed to serialize tool_types")?;

        let id: i64 =
            sqlx::query("INSERT INTO tool_groups (name, tool_types) VALUES (?1, ?2) RETURNING id")
                .bind(name)
                .bind(&tool_types_json)
                .fetch_one(&self.pool)
                .await
                .context(format!("Failed to create tool group: {}", name))?
                .get(0);

        self.get_group(id)
            .await?
            .context("Failed to retrieve created tool group")
    }

    pub async fn update_group(
        &self,
        id: i64,
        name: &str,
        tool_types: &[String],
    ) -> Result<Option<ToolGroup>> {
        let tool_types_json =
            serde_json::to_string(tool_types).context("Failed to serialize tool_types")?;

        let rows_affected = sqlx::query(
            "UPDATE tool_groups SET name = ?1, tool_types = ?2, updated_at = strftime('%s', 'now') WHERE id = ?3",
        )
        .bind(name)
        .bind(&tool_types_json)
        .bind(id)
        .execute(&self.pool)
        .await
        .context(format!("Failed to update tool group: {}", id))?
        .rows_affected();

        if rows_affected == 0 {
            return Ok(None);
        }

        self.get_group(id).await
    }

    pub async fn delete_group(&self, id: i64) -> Result<bool> {
        let rows_affected = sqlx::query("DELETE FROM tool_groups WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete tool group")?
            .rows_affected();

        Ok(rows_affected > 0)
    }
}
