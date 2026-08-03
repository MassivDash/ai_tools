use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::types::{ChromaDBResponse, Collection};
use actix_web::{get, web, HttpResponse, Result as ActixResult};

#[get("/api/chromadb/collections/{name}")]
pub async fn get_collection(
    path: web::Path<String>,
    chroma_address: web::Data<String>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    let client = match ChromaDBClient::new(chroma_address.as_str()) {
        Ok(c) => c,
        Err(e) => {
            println!("Failed to create ChromaDB client: {}", e);
            return Ok(
                HttpResponse::ServiceUnavailable().json(ChromaDBResponse::<Collection> {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                    message: None,
                }),
            );
        }
    };

    match client.get_collection(&name).await {
        Ok(collection) => Ok(HttpResponse::Ok().json(ChromaDBResponse {
            success: true,
            data: Some(collection),
            error: None,
            message: None,
        })),
        Err(e) => {
            println!("Failed to get collection: {}", e);
            Ok(
                HttpResponse::NotFound().json(ChromaDBResponse::<Collection> {
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
    use crate::test_support::{
        lock_chroma_endpoint, MockChroma, MockChromaCollection, MockChromaConfig,
        UNPARSEABLE_CHROMA_ENDPOINT,
    };
    use actix_web::{test, App};

    #[actix_web::test]
    async fn test_get_collection_returns_the_named_collection() {
        let chroma = MockChroma::start(MockChromaConfig::holding(vec![
            MockChromaCollection::new("notes")
                .with_metadata(&[("owner", "alice")])
                .with_count(11),
            MockChromaCollection::new("papers"),
        ]))
        .await;

        let guard = lock_chroma_endpoint();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(chroma.base_url.clone()))
                .service(get_collection),
        )
        .await;
        let resp = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/api/chromadb/collections/notes")
                .to_request(),
        )
        .await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: ChromaDBResponse<Collection> = test::read_body_json(resp).await;
        drop(guard);

        assert!(body.success);
        let collection = body.data.unwrap();
        assert_eq!(collection.name, "notes");
        assert_eq!(collection.count, Some(11));
        assert_eq!(collection.metadata.as_ref().unwrap()["owner"], "alice");

        // The handler asked for exactly the collection named in the path.
        assert!(chroma.requests()[0].path.ends_with("/collections/notes"));

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_get_collection_reports_an_unknown_name_as_404() {
        let chroma = MockChroma::start(MockChromaConfig::holding(vec![MockChromaCollection::new(
            "notes",
        )]))
        .await;

        let guard = lock_chroma_endpoint();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(chroma.base_url.clone()))
                .service(get_collection),
        )
        .await;
        let resp = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/api/chromadb/collections/missing")
                .to_request(),
        )
        .await;

        assert_eq!(resp.status().as_u16(), 404);
        let body: ChromaDBResponse<Collection> = test::read_body_json(resp).await;
        drop(guard);

        assert!(!body.success);
        assert!(body.data.is_none());
        assert!(body.error.unwrap().contains("Failed to get collection"));

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_get_collection_reports_an_unusable_address_as_503() {
        let _guard = lock_chroma_endpoint();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UNPARSEABLE_CHROMA_ENDPOINT.to_string()))
                .service(get_collection),
        )
        .await;
        let resp = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/api/chromadb/collections/notes")
                .to_request(),
        )
        .await;

        assert_eq!(resp.status().as_u16(), 503);
        let body: ChromaDBResponse<Collection> = test::read_body_json(resp).await;
        assert!(!body.success);
        assert!(body
            .error
            .unwrap()
            .contains("Failed to create ChromaDB client"));
    }
}
