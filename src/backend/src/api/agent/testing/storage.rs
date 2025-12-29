use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct TestSuite {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestQuestion {
    pub id: i64,
    pub suite_id: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Clone)]
pub struct TestingStorage {
    pool: SqlitePool,
}

impl TestingStorage {
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        let storage = Self { pool };
        storage.initialize().await?;
        Ok(storage)
    }

    async fn initialize(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS test_suites (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            )",
        )
        .execute(&self.pool)
        .await
        .context("Failed to create test_suites table")?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS test_questions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                suite_id TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                FOREIGN KEY (suite_id) REFERENCES test_suites(id) ON DELETE CASCADE
            )",
        )
        .execute(&self.pool)
        .await
        .context("Failed to create test_questions table")?;

        Ok(())
    }

    // --- Suites CRUD ---

    pub async fn get_suites(&self) -> Result<Vec<TestSuite>> {
        let rows = sqlx::query(
            "SELECT id, name, description, created_at FROM test_suites ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch test suites")?;

        let suites = rows
            .into_iter()
            .map(|row| TestSuite {
                id: row.get(0),
                name: row.get(1),
                description: row.get(2),
                created_at: row.get(3),
            })
            .collect();

        Ok(suites)
    }

    pub async fn create_suite(
        &self,
        name: String,
        description: Option<String>,
    ) -> Result<TestSuite> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();

        sqlx::query(
            "INSERT INTO test_suites (id, name, description, created_at) VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(&id)
        .bind(&name)
        .bind(&description)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create test suite")?;

        Ok(TestSuite {
            id,
            name,
            description,
            created_at: now,
        })
    }

    pub async fn update_suite(
        &self,
        id: &str,
        name: String,
        description: Option<String>,
    ) -> Result<()> {
        sqlx::query("UPDATE test_suites SET name = ?1, description = ?2 WHERE id = ?3")
            .bind(name)
            .bind(description)
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to update test suite")?;
        Ok(())
    }

    pub async fn delete_suite(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM test_suites WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete test suite")?;
        Ok(())
    }

    // --- Questions CRUD ---

    pub async fn get_questions(&self, suite_id: &str) -> Result<Vec<TestQuestion>> {
        let rows = sqlx::query(
            "SELECT id, suite_id, content, created_at FROM test_questions WHERE suite_id = ?1 ORDER BY created_at ASC",
        )
        .bind(suite_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch test questions")?;

        let questions = rows
            .into_iter()
            .map(|row| TestQuestion {
                id: row.get(0),
                suite_id: row.get(1),
                content: row.get(2),
                created_at: row.get(3),
            })
            .collect();

        Ok(questions)
    }

    pub async fn add_question(&self, suite_id: &str, content: String) -> Result<TestQuestion> {
        let now = chrono::Utc::now().timestamp();

        let id = sqlx::query(
            "INSERT INTO test_questions (suite_id, content, created_at) VALUES (?1, ?2, ?3) RETURNING id",
        )
        .bind(suite_id)
        .bind(&content)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .context("Failed to add question")?
        .get::<i64, _>(0);

        Ok(TestQuestion {
            id,
            suite_id: suite_id.to_string(),
            content,
            created_at: now,
        })
    }

    pub async fn update_question(&self, id: i64, content: String) -> Result<()> {
        sqlx::query("UPDATE test_questions SET content = ?1 WHERE id = ?2")
            .bind(content)
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to update question")?;
        Ok(())
    }

    pub async fn delete_question(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM test_questions WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete question")?;
        Ok(())
    }
}
