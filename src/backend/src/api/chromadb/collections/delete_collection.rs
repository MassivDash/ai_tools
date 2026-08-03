use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::types::ChromaDBResponse;
use actix_web::{delete, web, HttpResponse, Result as ActixResult};

#[delete("/api/chromadb/collections/{name}")]
pub async fn delete_collection(
    path: web::Path<String>,
    chroma_address: web::Data<String>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    let client = match ChromaDBClient::new(chroma_address.as_str()) {
        Ok(c) => c,
        Err(e) => {
            println!("Failed to create ChromaDB client: {}", e);
            return Ok(
                HttpResponse::ServiceUnavailable().json(ChromaDBResponse::<()> {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                    message: None,
                }),
            );
        }
    };

    match client.delete_collection(&name).await {
        Ok(_) => Ok(HttpResponse::Ok().json(ChromaDBResponse::<()> {
            success: true,
            data: None,
            error: None,
            message: Some(format!("Collection {} deleted successfully", name)),
        })),
        Err(e) => {
            println!("Failed to delete collection: {}", e);
            Ok(
                HttpResponse::InternalServerError().json(ChromaDBResponse::<()> {
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
    async fn test_delete_collection_removes_it_and_confirms_by_name() {
        let chroma = MockChroma::start(MockChromaConfig::holding(vec![
            MockChromaCollection::new("notes"),
            MockChromaCollection::new("papers"),
        ]))
        .await;

        let guard = lock_chroma_endpoint();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(chroma.base_url.clone()))
                .service(delete_collection),
        )
        .await;
        let resp = test::call_service(
            &app,
            test::TestRequest::delete()
                .uri("/api/chromadb/collections/notes")
                .to_request(),
        )
        .await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: ChromaDBResponse<()> = test::read_body_json(resp).await;
        drop(guard);

        assert!(body.success);
        assert_eq!(
            body.message.unwrap(),
            "Collection notes deleted successfully"
        );
        // Only the named collection went away.
        assert_eq!(chroma.collection_names(), vec!["papers".to_string()]);

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_delete_collection_reports_an_unknown_name_as_500() {
        let chroma = MockChroma::start(MockChromaConfig::empty()).await;

        let guard = lock_chroma_endpoint();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(chroma.base_url.clone()))
                .service(delete_collection),
        )
        .await;
        let resp = test::call_service(
            &app,
            test::TestRequest::delete()
                .uri("/api/chromadb/collections/missing")
                .to_request(),
        )
        .await;

        assert_eq!(resp.status().as_u16(), 500);
        let body: ChromaDBResponse<()> = test::read_body_json(resp).await;
        drop(guard);

        assert!(!body.success);
        assert!(body.message.is_none());
        assert!(body.error.unwrap().contains("Failed to delete collection"));

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_delete_collection_reports_an_unusable_address_as_503() {
        let _guard = lock_chroma_endpoint();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UNPARSEABLE_CHROMA_ENDPOINT.to_string()))
                .service(delete_collection),
        )
        .await;
        let resp = test::call_service(
            &app,
            test::TestRequest::delete()
                .uri("/api/chromadb/collections/notes")
                .to_request(),
        )
        .await;

        assert_eq!(resp.status().as_u16(), 503);
        let body: ChromaDBResponse<()> = test::read_body_json(resp).await;
        assert!(!body.success);
        assert!(body
            .error
            .unwrap()
            .contains("Failed to create ChromaDB client"));
    }
}
