use crate::api::pageindex::types::{PageIndexDocument, PageIndexSummary};
use anyhow::{Context, Result};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Row, SqlitePool,
};
use std::path::Path;

/// SQLite-based storage for PageIndex documents
pub struct PageIndexStorage {
    pool: SqlitePool,
}

impl PageIndexStorage {
    /// Create a new PageIndex storage, sharing the same SQLite file as other storages.
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref();
        let db_path_str = db_path.to_str().unwrap_or("");
        let is_memory = db_path_str == ":memory:";

        let (db_path_for_connection, display_path) = if is_memory {
            println!("💾 Connecting to SQLite in-memory database for pageindex");
            (":memory:".to_string(), ":memory:".to_string())
        } else {
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent).context("Failed to create database directory")?;
            }

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
                "💾 Connecting to SQLite database for pageindex at: {}",
                display
            );
            (absolute_path.to_str().unwrap().to_string(), display)
        };

        let options = SqliteConnectOptions::new()
            .filename(&db_path_for_connection)
            .create_if_missing(true);

        // SQLite ":memory:" databases are private per-connection, so a pool with more
        // than one connection would silently see an empty database on some queries.
        // Pin the pool to a single connection in that case (only relevant for tests).
        let pool = if is_memory {
            SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(options)
                .await
        } else {
            SqlitePool::connect_with(options).await
        }
        .context(format!(
            "Failed to connect to SQLite database at: {}",
            display_path
        ))?;

        println!("📋 Creating pageindex_documents table if it doesn't exist...");
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS pageindex_documents (
                id TEXT PRIMARY KEY,
                filename TEXT NOT NULL,
                title TEXT NOT NULL,
                status TEXT NOT NULL,
                page_count INTEGER,
                node_count INTEGER,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
                error TEXT
            )",
        )
        .execute(&pool)
        .await
        .context("Failed to create pageindex_documents table")?;
        println!("✅ pageindex_documents table created/verified");

        Ok(Self { pool })
    }

    /// Insert a new document row in the "processing" state.
    pub async fn insert_pending(&self, id: &str, filename: &str, title: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO pageindex_documents (id, filename, title, status)
             VALUES (?1, ?2, ?3, 'processing')",
        )
        .bind(id)
        .bind(filename)
        .bind(title)
        .execute(&self.pool)
        .await
        .context("Failed to insert pending pageindex document")?;
        Ok(())
    }

    /// Mark a document as ready, recording its page/node counts.
    pub async fn mark_ready(&self, id: &str, page_count: u32, node_count: u32) -> Result<()> {
        sqlx::query(
            "UPDATE pageindex_documents
             SET status = 'ready', page_count = ?2, node_count = ?3, error = NULL
             WHERE id = ?1",
        )
        .bind(id)
        .bind(page_count)
        .bind(node_count)
        .execute(&self.pool)
        .await
        .context("Failed to mark pageindex document ready")?;
        Ok(())
    }

    /// Mark a document as errored, recording the error message.
    pub async fn mark_error(&self, id: &str, error: &str) -> Result<()> {
        sqlx::query(
            "UPDATE pageindex_documents
             SET status = 'error', error = ?2
             WHERE id = ?1",
        )
        .bind(id)
        .bind(error)
        .execute(&self.pool)
        .await
        .context("Failed to mark pageindex document error")?;
        Ok(())
    }

    /// List all documents, most recently created first.
    pub async fn list_documents(&self) -> Result<Vec<PageIndexDocument>> {
        let rows = sqlx::query(
            "SELECT id, filename, title, status, page_count, node_count, created_at, error
             FROM pageindex_documents
             ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list pageindex documents")?;

        Ok(rows
            .into_iter()
            .map(|row| PageIndexDocument {
                id: row.get(0),
                filename: row.get(1),
                title: row.get(2),
                status: row.get(3),
                page_count: row.get::<Option<i64>, _>(4).map(|v| v as u32),
                node_count: row.get::<Option<i64>, _>(5).map(|v| v as u32),
                created_at: row.get(6),
                error: row.get(7),
            })
            .collect())
    }

    /// List only "ready" documents as cheap summaries (feeds the agent tool).
    pub async fn list_summaries(&self) -> Result<Vec<PageIndexSummary>> {
        let rows = sqlx::query(
            "SELECT id, filename, title FROM pageindex_documents
             WHERE status = 'ready'
             ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list pageindex summaries")?;

        Ok(rows
            .into_iter()
            .map(|row| PageIndexSummary {
                id: row.get(0),
                filename: row.get(1),
                title: row.get(2),
            })
            .collect())
    }

    /// Fetch a single document by id.
    pub async fn get_document(&self, id: &str) -> Result<Option<PageIndexDocument>> {
        let row = sqlx::query(
            "SELECT id, filename, title, status, page_count, node_count, created_at, error
             FROM pageindex_documents
             WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch pageindex document")?;

        Ok(row.map(|row| PageIndexDocument {
            id: row.get(0),
            filename: row.get(1),
            title: row.get(2),
            status: row.get(3),
            page_count: row.get::<Option<i64>, _>(4).map(|v| v as u32),
            node_count: row.get::<Option<i64>, _>(5).map(|v| v as u32),
            created_at: row.get(6),
            error: row.get(7),
        }))
    }

    /// Delete a document row (DB only - callers are responsible for removing files on disk).
    pub async fn delete_document(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM pageindex_documents WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete pageindex document")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_insert_and_list_documents() {
        let storage = PageIndexStorage::new(":memory:").await.unwrap();
        storage
            .insert_pending("id1", "book.pdf", "Book")
            .await
            .unwrap();

        let docs = storage.list_documents().await.unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].status, "processing");
        assert!(docs[0].page_count.is_none());

        // Not ready yet - should not appear in summaries
        let summaries = storage.list_summaries().await.unwrap();
        assert!(summaries.is_empty());

        storage.mark_ready("id1", 100, 12).await.unwrap();
        let docs = storage.list_documents().await.unwrap();
        assert_eq!(docs[0].status, "ready");
        assert_eq!(docs[0].page_count, Some(100));
        assert_eq!(docs[0].node_count, Some(12));

        let summaries = storage.list_summaries().await.unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, "id1");

        let doc = storage.get_document("id1").await.unwrap().unwrap();
        assert_eq!(doc.title, "Book");

        storage.delete_document("id1").await.unwrap();
        assert!(storage.get_document("id1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mark_error() {
        let storage = PageIndexStorage::new(":memory:").await.unwrap();
        storage
            .insert_pending("id2", "book2.pdf", "Book 2")
            .await
            .unwrap();
        storage.mark_error("id2", "boom").await.unwrap();

        let doc = storage.get_document("id2").await.unwrap().unwrap();
        assert_eq!(doc.status, "error");
        assert_eq!(doc.error.as_deref(), Some("boom"));
    }
}
