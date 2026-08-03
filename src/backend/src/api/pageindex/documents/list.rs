use crate::api::pageindex::storage::PageIndexStorage;
use actix_web::{get, web, HttpResponse, Result as ActixResult};
use std::sync::Arc;

#[get("/api/pageindex/documents")]
pub async fn list_documents(
    storage: web::Data<Arc<PageIndexStorage>>,
) -> ActixResult<HttpResponse> {
    match storage.list_documents().await {
        Ok(documents) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "documents": documents
        }))),
        Err(e) => {
            println!("⚠️ PageIndex: failed to list documents: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": e.to_string()
            })))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    async fn storage_with_documents() -> Arc<PageIndexStorage> {
        let storage = PageIndexStorage::new(":memory:").await.unwrap();
        storage
            .insert_pending("doc-1", "one.pdf", "One")
            .await
            .unwrap();
        storage
            .insert_pending("doc-2", "two.pdf", "Two")
            .await
            .unwrap();
        storage.mark_ready("doc-2", 42, 7).await.unwrap();
        Arc::new(storage)
    }

    #[actix_web::test]
    async fn test_list_documents_empty() {
        let storage = Arc::new(PageIndexStorage::new(":memory:").await.unwrap());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(storage))
                .service(list_documents),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/pageindex/documents")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["documents"].as_array().unwrap().len(), 0);
    }

    #[actix_web::test]
    async fn test_list_documents_returns_full_records() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(storage_with_documents().await))
                .service(list_documents),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/pageindex/documents")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);

        let docs = body["documents"].as_array().unwrap();
        assert_eq!(docs.len(), 2);

        let ready = docs
            .iter()
            .find(|d| d["id"] == "doc-2")
            .expect("doc-2 should be listed");
        assert_eq!(ready["status"], "ready");
        assert_eq!(ready["filename"], "two.pdf");
        assert_eq!(ready["title"], "Two");
        assert_eq!(ready["page_count"], 42);
        assert_eq!(ready["node_count"], 7);
        assert!(ready["error"].is_null());

        let pending = docs
            .iter()
            .find(|d| d["id"] == "doc-1")
            .expect("doc-1 should be listed");
        assert_eq!(pending["status"], "processing");
        assert!(pending["page_count"].is_null());
    }

    #[actix_web::test]
    async fn test_list_documents_storage_failure_returns_500() {
        let storage = PageIndexStorage::new(":memory:").await.unwrap();
        storage.drop_table_for_tests().await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(storage)))
                .service(list_documents),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/pageindex/documents")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 500);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"], "Failed to list pageindex documents");
    }
}
