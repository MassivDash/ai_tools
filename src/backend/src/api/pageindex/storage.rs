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

    /// Drop the backing table so that every subsequent query fails.
    ///
    /// Used by the handler tests to exercise the `Err(..)` arms that map storage
    /// failures onto `500` responses; there is no other way to make a healthy
    /// in-memory database fail on demand.
    #[cfg(test)]
    pub(crate) async fn drop_table_for_tests(&self) {
        sqlx::query("DROP TABLE pageindex_documents")
            .execute(&self.pool)
            .await
            .expect("Failed to drop pageindex_documents table");
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

    #[tokio::test]
    async fn test_get_document_returns_none_for_unknown_id() {
        let storage = PageIndexStorage::new(":memory:").await.unwrap();
        assert!(storage.get_document("nope").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_unknown_document_is_not_an_error() {
        let storage = PageIndexStorage::new(":memory:").await.unwrap();
        assert!(storage.delete_document("nope").await.is_ok());
    }

    #[tokio::test]
    async fn test_duplicate_id_is_rejected_by_primary_key() {
        let storage = PageIndexStorage::new(":memory:").await.unwrap();
        storage.insert_pending("dup", "a.pdf", "A").await.unwrap();

        assert!(storage.insert_pending("dup", "b.pdf", "B").await.is_err());
        assert_eq!(storage.list_documents().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_mark_ready_clears_a_previous_error() {
        let storage = PageIndexStorage::new(":memory:").await.unwrap();
        storage.insert_pending("retry", "a.pdf", "A").await.unwrap();
        storage.mark_error("retry", "first attempt").await.unwrap();

        storage.mark_ready("retry", 10, 3).await.unwrap();

        let doc = storage.get_document("retry").await.unwrap().unwrap();
        assert_eq!(doc.status, "ready");
        assert!(doc.error.is_none());
        assert_eq!(doc.page_count, Some(10));
    }

    #[tokio::test]
    async fn test_mark_error_keeps_the_page_counts_from_an_earlier_success() {
        let storage = PageIndexStorage::new(":memory:").await.unwrap();
        storage.insert_pending("mixed", "a.pdf", "A").await.unwrap();
        storage.mark_ready("mixed", 7, 2).await.unwrap();

        storage.mark_error("mixed", "later failure").await.unwrap();

        let doc = storage.get_document("mixed").await.unwrap().unwrap();
        assert_eq!(doc.status, "error");
        assert_eq!(doc.error.as_deref(), Some("later failure"));
        assert_eq!(doc.page_count, Some(7));
        assert_eq!(doc.node_count, Some(2));
    }

    #[tokio::test]
    async fn test_marking_an_unknown_id_is_a_no_op() {
        let storage = PageIndexStorage::new(":memory:").await.unwrap();

        storage.mark_ready("ghost", 1, 1).await.unwrap();
        storage.mark_error("ghost", "boom").await.unwrap();

        assert!(storage.list_documents().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_summaries_only_include_ready_documents() {
        let storage = PageIndexStorage::new(":memory:").await.unwrap();
        storage
            .insert_pending("p", "p.pdf", "Pending")
            .await
            .unwrap();
        storage
            .insert_pending("e", "e.pdf", "Errored")
            .await
            .unwrap();
        storage.insert_pending("r", "r.pdf", "Ready").await.unwrap();
        storage.mark_error("e", "boom").await.unwrap();
        storage.mark_ready("r", 5, 1).await.unwrap();

        let summaries = storage.list_summaries().await.unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, "r");
        assert_eq!(summaries[0].filename, "r.pdf");
        assert_eq!(summaries[0].title, "Ready");
    }

    #[tokio::test]
    async fn test_delete_removes_only_the_named_document() {
        let storage = PageIndexStorage::new(":memory:").await.unwrap();
        storage.insert_pending("keep", "k.pdf", "K").await.unwrap();
        storage.insert_pending("drop", "d.pdf", "D").await.unwrap();

        storage.delete_document("drop").await.unwrap();

        let docs = storage.list_documents().await.unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id, "keep");
    }

    #[tokio::test]
    async fn test_file_backed_storage_creates_its_directory_and_persists() {
        let dir = tempfile::tempdir().unwrap();
        // The parent directory does not exist yet - `new` must create it.
        let db_path = dir.path().join("nested").join("pageindex.db");

        let storage = PageIndexStorage::new(&db_path).await.unwrap();
        storage
            .insert_pending("on-disk", "a.pdf", "A")
            .await
            .unwrap();
        storage.mark_ready("on-disk", 3, 1).await.unwrap();
        drop(storage);

        assert!(db_path.exists());

        // Re-opening the existing file keeps the rows.
        let reopened = PageIndexStorage::new(&db_path).await.unwrap();
        let docs = reopened.list_documents().await.unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id, "on-disk");
        assert_eq!(docs[0].status, "ready");
    }
}
