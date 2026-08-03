use crate::api::pageindex::storage::PageIndexStorage;
use crate::api::pageindex::types::PageIndexNode;
use actix_web::{get, web, HttpResponse, Result as ActixResult};
use std::sync::Arc;

#[get("/api/pageindex/documents/{id}")]
pub async fn get_document(
    path: web::Path<String>,
    storage: web::Data<Arc<PageIndexStorage>>,
) -> ActixResult<HttpResponse> {
    let id = path.into_inner();

    let document = match storage.get_document(&id).await {
        Ok(Some(doc)) => doc,
        Ok(None) => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "success": false,
                "error": format!("Document '{}' not found", id)
            })));
        }
        Err(e) => {
            println!("⚠️ PageIndex: failed to fetch document '{}': {}", id, e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": e.to_string()
            })));
        }
    };

    // While the document is still processing (or if something went wrong writing it),
    // tree.json may not exist yet - that's not an error, just an empty tree.
    let tree_path = std::path::Path::new("./public/pageindex")
        .join(&id)
        .join("tree.json");

    let tree: Vec<PageIndexNode> = if tree_path.exists() {
        match tokio::fs::read_to_string(&tree_path).await {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "document": document,
        "tree": tree
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    /// Guards a `./public/pageindex/{id}` directory so each test cleans up after
    /// itself - the handler resolves the tree path relative to the process CWD.
    struct DocDir {
        id: String,
    }

    impl DocDir {
        fn new() -> Self {
            let id = format!("test-{}", uuid::Uuid::new_v4());
            std::fs::create_dir_all(Self::path_for(&id)).unwrap();
            Self { id }
        }

        fn path_for(id: &str) -> std::path::PathBuf {
            std::path::Path::new("./public/pageindex").join(id)
        }

        fn path(&self) -> std::path::PathBuf {
            Self::path_for(&self.id)
        }

        fn write_tree(&self, contents: &str) {
            std::fs::write(self.path().join("tree.json"), contents).unwrap();
        }
    }

    impl Drop for DocDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.path());
        }
    }

    async fn app_storage(id: &str) -> Arc<PageIndexStorage> {
        let storage = PageIndexStorage::new(":memory:").await.unwrap();
        storage
            .insert_pending(id, "book.pdf", "Book")
            .await
            .unwrap();
        Arc::new(storage)
    }

    async fn call(storage: Arc<PageIndexStorage>, id: &str) -> serde_json::Value {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(storage))
                .service(get_document),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/pageindex/documents/{}", id))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 200);
        test::read_body_json(resp).await
    }

    #[actix_web::test]
    async fn test_get_document_not_found() {
        let storage = Arc::new(PageIndexStorage::new(":memory:").await.unwrap());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(storage))
                .service(get_document),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/pageindex/documents/missing")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 404);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"], "Document 'missing' not found");
    }

    #[actix_web::test]
    async fn test_get_document_with_no_tree_on_disk_returns_an_empty_tree() {
        // Still processing: tree.json has not been written yet, which is not an error.
        let storage = app_storage("no-tree-yet").await;

        let body = call(storage, "no-tree-yet").await;

        assert_eq!(body["success"], true);
        assert_eq!(body["document"]["id"], "no-tree-yet");
        assert_eq!(body["document"]["status"], "processing");
        assert_eq!(body["tree"].as_array().unwrap().len(), 0);
    }

    #[actix_web::test]
    async fn test_get_document_returns_the_tree_from_disk() {
        let dir = DocDir::new();
        dir.write_tree(
            r#"[{"id":"n1","title":"Chapter 1","page_start":1,"page_end":9,"summary":"intro",
                 "children":[{"id":"n2","title":"1.1","page_start":2,"page_end":4,
                              "summary":"details","children":[]}]}]"#,
        );

        let storage = app_storage(&dir.id).await;
        let body = call(storage, &dir.id).await;

        assert_eq!(body["success"], true);
        let tree = body["tree"].as_array().unwrap();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0]["title"], "Chapter 1");
        assert_eq!(tree[0]["page_end"], 9);
        assert_eq!(tree[0]["children"][0]["title"], "1.1");
    }

    #[actix_web::test]
    async fn test_get_document_tolerates_a_corrupt_tree_file() {
        let dir = DocDir::new();
        dir.write_tree("this is not json");

        let storage = app_storage(&dir.id).await;
        let body = call(storage, &dir.id).await;

        // A broken tree.json degrades to an empty tree rather than failing the request.
        assert_eq!(body["success"], true);
        assert_eq!(body["tree"].as_array().unwrap().len(), 0);
    }

    #[actix_web::test]
    async fn test_get_document_tolerates_a_tree_path_that_is_a_directory() {
        let dir = DocDir::new();
        // tree.json exists but cannot be read as a file.
        std::fs::create_dir_all(dir.path().join("tree.json")).unwrap();

        let storage = app_storage(&dir.id).await;
        let body = call(storage, &dir.id).await;

        assert_eq!(body["success"], true);
        assert_eq!(body["tree"].as_array().unwrap().len(), 0);
    }

    #[actix_web::test]
    async fn test_get_document_storage_failure_returns_500() {
        let storage = PageIndexStorage::new(":memory:").await.unwrap();
        storage.drop_table_for_tests().await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(get_document),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/pageindex/documents/anything")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 500);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"], "Failed to fetch pageindex document");
    }
}
