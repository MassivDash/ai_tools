use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::types::ChromaDBResponse;
use actix_web::{get, web, HttpResponse, Result as ActixResult};

#[get("/api/chromadb/collections")]
pub async fn get_collections(chroma_address: web::Data<String>) -> ActixResult<HttpResponse> {
    let client = match ChromaDBClient::new(chroma_address.as_str()) {
        Ok(c) => c,
        Err(e) => {
            println!("Failed to create ChromaDB client: {}", e);
            return Ok(
                HttpResponse::ServiceUnavailable().json(ChromaDBResponse::<Vec<()>> {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                    message: None,
                }),
            );
        }
    };

    match client.list_collections().await {
        Ok(collections) => Ok(HttpResponse::Ok().json(ChromaDBResponse {
            success: true,
            data: Some(collections),
            error: None,
            message: None,
        })),
        Err(e) => {
            println!("Failed to list collections: {}", e);
            Ok(
                HttpResponse::InternalServerError().json(ChromaDBResponse::<Vec<()>> {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                    message: None,
                }),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::chromadb::types::Collection;
    use crate::test_support::{
        lock_chroma_endpoint, MockChroma, MockChromaCollection, MockChromaConfig,
        UNPARSEABLE_CHROMA_ENDPOINT,
    };
    use actix_web::{test, App};

    /// The one request this handler serves.
    fn list_request() -> test::TestRequest {
        test::TestRequest::get().uri("/api/chromadb/collections")
    }

    #[actix_web::test]
    async fn test_get_collections_returns_every_collection() {
        let chroma = MockChroma::start(MockChromaConfig::holding(vec![
            MockChromaCollection::new("notes")
                .with_metadata(&[("embedding_model", "nomic-embed-text")])
                .with_count(4),
            MockChromaCollection::new("papers"),
        ]))
        .await;

        let guard = lock_chroma_endpoint();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(chroma.base_url.clone()))
                .service(get_collections),
        )
        .await;
        let resp = test::call_service(&app, list_request().to_request()).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: ChromaDBResponse<Vec<Collection>> = test::read_body_json(resp).await;
        drop(guard);

        assert!(body.success);
        assert!(body.error.is_none());
        let collections = body.data.unwrap();
        assert_eq!(collections.len(), 2);
        assert_eq!(collections[0].name, "notes");
        assert_eq!(collections[0].count, Some(4));
        assert_eq!(
            collections[0].metadata.as_ref().unwrap()["embedding_model"],
            "nomic-embed-text"
        );
        assert_eq!(collections[1].name, "papers");

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_get_collections_returns_an_empty_list() {
        let chroma = MockChroma::start(MockChromaConfig::empty()).await;

        let guard = lock_chroma_endpoint();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(chroma.base_url.clone()))
                .service(get_collections),
        )
        .await;
        let resp = test::call_service(&app, list_request().to_request()).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: ChromaDBResponse<Vec<Collection>> = test::read_body_json(resp).await;
        drop(guard);

        assert!(body.success);
        assert!(body.data.unwrap().is_empty());

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_get_collections_reports_a_server_failure_as_500() {
        let chroma = MockChroma::start(MockChromaConfig {
            list_status: Some(400),
            ..MockChromaConfig::empty()
        })
        .await;

        let guard = lock_chroma_endpoint();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(chroma.base_url.clone()))
                .service(get_collections),
        )
        .await;
        let resp = test::call_service(&app, list_request().to_request()).await;

        assert_eq!(resp.status().as_u16(), 500);
        let body: ChromaDBResponse<Vec<Collection>> = test::read_body_json(resp).await;
        drop(guard);

        assert!(!body.success);
        assert!(body.data.is_none());
        assert!(body.error.unwrap().contains("Failed to list collections"));

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_get_collections_reports_an_unusable_address_as_503() {
        let _guard = lock_chroma_endpoint();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UNPARSEABLE_CHROMA_ENDPOINT.to_string()))
                .service(get_collections),
        )
        .await;
        let resp = test::call_service(&app, list_request().to_request()).await;

        assert_eq!(resp.status().as_u16(), 503);
        let body: ChromaDBResponse<Vec<Collection>> = test::read_body_json(resp).await;
        assert!(!body.success);
        assert!(body
            .error
            .unwrap()
            .contains("Failed to create ChromaDB client"));
    }
}
