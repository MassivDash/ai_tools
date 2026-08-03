use crate::api::pageindex::storage::PageIndexStorage;
use actix_web::{delete, web, HttpResponse, Result as ActixResult};
use std::sync::Arc;

#[delete("/api/pageindex/documents/{id}")]
pub async fn delete_document(
    path: web::Path<String>,
    storage: web::Data<Arc<PageIndexStorage>>,
) -> ActixResult<HttpResponse> {
    let id = path.into_inner();

    if let Err(e) = storage.delete_document(&id).await {
        println!("⚠️ PageIndex: failed to delete document '{}': {}", id, e);
        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": e.to_string()
        })));
    }

    let dir = std::path::Path::new("./public/pageindex").join(&id);
    if dir.exists() {
        if let Err(e) = tokio::fs::remove_dir_all(&dir).await {
            println!(
                "⚠️ PageIndex: failed to remove directory {:?} for document '{}': {}",
                dir, id, e
            );
        }
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({ "success": true })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    fn doc_dir(id: &str) -> std::path::PathBuf {
        std::path::Path::new("./public/pageindex").join(id)
    }

    async fn call_delete(storage: Arc<PageIndexStorage>, id: &str) -> (u16, serde_json::Value) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(storage))
                .service(delete_document),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/pageindex/documents/{}", id))
            .to_request();
        let resp = test::call_service(&app, req).await;
        let status = resp.status().as_u16();
        (status, test::read_body_json(resp).await)
    }

    #[actix_web::test]
    async fn test_delete_removes_the_row_and_the_on_disk_directory() {
        let id = format!("test-{}", uuid::Uuid::new_v4());
        let dir = doc_dir(&id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("tree.json"), "[]").unwrap();
        std::fs::write(dir.join("source.pdf"), b"%PDF-1.4").unwrap();

        let storage = PageIndexStorage::new(":memory:").await.unwrap();
        storage
            .insert_pending(&id, "book.pdf", "Book")
            .await
            .unwrap();
        let storage = Arc::new(storage);

        let (status, body) = call_delete(storage.clone(), &id).await;

        assert_eq!(status, 200);
        assert_eq!(body["success"], true);
        assert!(storage.get_document(&id).await.unwrap().is_none());
        assert!(!dir.exists(), "the document directory should be removed");
    }

    #[actix_web::test]
    async fn test_delete_succeeds_when_there_is_no_directory_on_disk() {
        let storage = PageIndexStorage::new(":memory:").await.unwrap();
        storage
            .insert_pending("db-only", "book.pdf", "Book")
            .await
            .unwrap();
        let storage = Arc::new(storage);

        let (status, body) = call_delete(storage.clone(), "db-only").await;

        assert_eq!(status, 200);
        assert_eq!(body["success"], true);
        assert!(storage.get_document("db-only").await.unwrap().is_none());
    }

    #[actix_web::test]
    async fn test_delete_of_an_unknown_id_reports_success() {
        let storage = Arc::new(PageIndexStorage::new(":memory:").await.unwrap());

        // The DELETE matches no rows, which the handler treats as success.
        let (status, body) = call_delete(storage, "never-existed").await;

        assert_eq!(status, 200);
        assert_eq!(body["success"], true);
    }

    #[actix_web::test]
    async fn test_delete_leaves_other_documents_alone() {
        let storage = PageIndexStorage::new(":memory:").await.unwrap();
        storage.insert_pending("keep", "k.pdf", "K").await.unwrap();
        storage.insert_pending("drop", "d.pdf", "D").await.unwrap();
        let storage = Arc::new(storage);

        let (status, _) = call_delete(storage.clone(), "drop").await;

        assert_eq!(status, 200);
        let remaining = storage.list_documents().await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, "keep");
    }

    #[actix_web::test]
    async fn test_delete_storage_failure_returns_500_and_keeps_the_directory() {
        let id = format!("test-{}", uuid::Uuid::new_v4());
        let dir = doc_dir(&id);
        std::fs::create_dir_all(&dir).unwrap();

        let storage = PageIndexStorage::new(":memory:").await.unwrap();
        storage.drop_table_for_tests().await;

        let (status, body) = call_delete(Arc::new(storage), &id).await;

        assert_eq!(status, 500);
        assert_eq!(body["success"], false);
        assert_eq!(body["error"], "Failed to delete pageindex document");
        // The handler bails out before touching the filesystem.
        assert!(dir.exists());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
