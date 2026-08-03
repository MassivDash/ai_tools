use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::types::{ChromaDBResponse, Collection, DistanceMetric};
use actix_web::{post, web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct CreateCollectionRequest {
    pub name: String,
    pub metadata: Option<HashMap<String, String>>,
    #[serde(default)]
    pub distance_metric: Option<DistanceMetric>,
    #[serde(default)]
    pub embedding_model: Option<String>,
}

#[post("/api/chromadb/collections")]
pub async fn create_collection(
    req: web::Json<CreateCollectionRequest>,
    chroma_address: web::Data<String>,
) -> ActixResult<HttpResponse> {
    println!(
        "📝 Creating collection request: name={}, metadata={:?}, distance_metric={:?}, embedding_model={:?}",
        req.name, req.metadata, req.distance_metric, req.embedding_model
    );

    // Validate collection name
    let collection_name = req.name.trim();
    if collection_name.is_empty() {
        return Ok(
            HttpResponse::BadRequest().json(ChromaDBResponse::<Collection> {
                success: false,
                data: None,
                error: Some("Collection name cannot be empty".to_string()),
                message: None,
            }),
        );
    }

    // Basic validation - ChromaDB will handle more specific validation
    // Just ensure it's not empty and not too long
    if collection_name.len() > 100 {
        return Ok(
            HttpResponse::BadRequest().json(ChromaDBResponse::<Collection> {
                success: false,
                data: None,
                error: Some("Collection name is too long (max 100 characters)".to_string()),
                message: None,
            }),
        );
    }

    // ChromaDB collection names cannot contain spaces or special characters
    // They must be valid identifiers (alphanumeric, underscores, hyphens)
    // Replace spaces with underscores and remove invalid characters
    let sanitized_name: String = collection_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_' // Replace spaces and other special chars with underscore
            }
        })
        .collect();

    // Ensure it doesn't start or end with underscore/hyphen
    let sanitized_name = sanitized_name.trim_matches(|c| c == '_' || c == '-');

    if sanitized_name.is_empty() {
        return Ok(
            HttpResponse::BadRequest().json(ChromaDBResponse::<Collection> {
                success: false,
                data: None,
                error: Some("Collection name contains only invalid characters".to_string()),
                message: None,
            }),
        );
    }

    // If name was sanitized, log it
    if sanitized_name != collection_name {
        println!(
            "⚠️ Collection name sanitized: '{}' -> '{}'",
            collection_name, sanitized_name
        );
    }

    let client = match ChromaDBClient::new(chroma_address.as_str()) {
        Ok(c) => c,
        Err(e) => {
            println!("Failed to create ChromaDB client: {}", e);
            return Ok(
                HttpResponse::ServiceUnavailable().json(ChromaDBResponse::<Collection> {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to connect to ChromaDB: {}", e)),
                    message: None,
                }),
            );
        }
    };

    println!(
        "✅ ChromaDB client created, attempting to create collection: {} (sanitized: {})",
        collection_name, sanitized_name
    );

    // Merge distance_metric into metadata if provided
    // ChromaDB accepts distance metric via metadata key "hnsw:space" or "distance_metric"
    let mut metadata = req.metadata.clone().unwrap_or_default();
    if let Some(metric) = &req.distance_metric {
        let metric_str = match metric {
            DistanceMetric::Cosine => "cosine",
            DistanceMetric::L2 => "l2",
            DistanceMetric::Ip => "ip",
        };
        metadata.insert("hnsw:space".to_string(), metric_str.to_string());
        println!("🔧 Setting distance metric to: {}", metric_str);
    }

    if let Some(model) = &req.embedding_model {
        metadata.insert("embedding_model".to_string(), model.clone());
        println!("🔧 Setting embedding model to: {}", model);
    }

    match client
        .create_collection(sanitized_name, Some(metadata))
        .await
    {
        Ok(collection) => {
            println!("✅ Collection created successfully: {}", collection.name);
            Ok(HttpResponse::Ok().json(ChromaDBResponse {
                success: true,
                data: Some(collection),
                error: None,
                message: None,
            }))
        }
        Err(e) => {
            println!("Failed to create collection '{}': {}", req.name, e);
            println!("   Error details: {:?}", e);
            Ok(
                HttpResponse::InternalServerError().json(ChromaDBResponse::<Collection> {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to create collection: {}", e)),
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
    use actix_web::{test, web, App};
    use std::collections::HashMap;

    fn post(request: CreateCollectionRequest) -> test::TestRequest {
        test::TestRequest::post()
            .uri("/api/chromadb/collections")
            .set_json(&request)
    }

    fn named(name: &str) -> CreateCollectionRequest {
        CreateCollectionRequest {
            name: name.to_string(),
            metadata: None,
            distance_metric: None,
            embedding_model: None,
        }
    }

    /// Names are rejected before a client is ever built, so these cases need no
    /// server at all - the address is deliberately unusable to prove that.
    async fn reject(request: CreateCollectionRequest) -> ChromaDBResponse<Collection> {
        let _guard = lock_chroma_endpoint();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UNPARSEABLE_CHROMA_ENDPOINT.to_string()))
                .service(create_collection),
        )
        .await;

        let resp = test::call_service(&app, post(request).to_request()).await;
        assert_eq!(resp.status().as_u16(), 400);
        test::read_body_json(resp).await
    }

    #[actix_web::test]
    async fn test_create_collection_empty_name() {
        let body = reject(named("   ")).await;

        assert!(!body.success);
        assert_eq!(body.error.unwrap(), "Collection name cannot be empty");
    }

    #[actix_web::test]
    async fn test_create_collection_name_too_long() {
        let body = reject(named(&"a".repeat(101))).await;

        assert!(!body.success);
        assert_eq!(
            body.error.unwrap(),
            "Collection name is too long (max 100 characters)"
        );
    }

    #[actix_web::test]
    async fn test_create_collection_name_of_only_invalid_characters() {
        // Every character is replaced with '_', and the result is then trimmed
        // away entirely.
        let body = reject(named("!!!")).await;

        assert!(!body.success);
        assert_eq!(
            body.error.unwrap(),
            "Collection name contains only invalid characters"
        );
    }

    #[actix_web::test]
    async fn test_create_collection_accepts_a_name_of_exactly_the_limit() {
        let chroma = MockChroma::start(MockChromaConfig::empty()).await;
        let name = "a".repeat(100);

        let guard = lock_chroma_endpoint();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(chroma.base_url.clone()))
                .service(create_collection),
        )
        .await;
        let resp = test::call_service(&app, post(named(&name)).to_request()).await;

        assert_eq!(resp.status().as_u16(), 200);
        drop(guard);
        assert_eq!(chroma.collection_names(), vec![name]);

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_create_collection_sanitises_the_name_before_sending_it() {
        let chroma = MockChroma::start(MockChromaConfig::empty()).await;

        let guard = lock_chroma_endpoint();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(chroma.base_url.clone()))
                .service(create_collection),
        )
        .await;
        let resp =
            test::call_service(&app, post(named("  my notes & papers!  ")).to_request()).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: ChromaDBResponse<Collection> = test::read_body_json(resp).await;
        drop(guard);

        // Spaces and punctuation become underscores; leading and trailing
        // underscores are then trimmed off.
        assert!(body.success);
        assert_eq!(body.data.unwrap().name, "my_notes___papers");
        assert_eq!(
            chroma.requests()[0].body.as_ref().unwrap()["name"],
            "my_notes___papers"
        );

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_create_collection_folds_the_distance_metric_and_model_into_metadata() {
        let chroma = MockChroma::start(MockChromaConfig::empty()).await;

        let guard = lock_chroma_endpoint();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(chroma.base_url.clone()))
                .service(create_collection),
        )
        .await;
        let resp = test::call_service(
            &app,
            post(CreateCollectionRequest {
                name: "notes".to_string(),
                metadata: Some(HashMap::from([(
                    "description".to_string(),
                    "test collection".to_string(),
                )])),
                distance_metric: Some(DistanceMetric::Cosine),
                embedding_model: Some("nomic-embed-text".to_string()),
            })
            .to_request(),
        )
        .await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: ChromaDBResponse<Collection> = test::read_body_json(resp).await;
        drop(guard);

        let sent = chroma.requests()[0].body.clone().unwrap();
        assert_eq!(sent["metadata"]["description"], "test collection");
        assert_eq!(sent["metadata"]["hnsw:space"], "cosine");
        assert_eq!(sent["metadata"]["embedding_model"], "nomic-embed-text");

        let created = body.data.unwrap();
        assert_eq!(created.count, Some(0));
        assert_eq!(created.metadata.as_ref().unwrap()["hnsw:space"], "cosine");

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_create_collection_maps_each_distance_metric_to_its_wire_name() {
        for (metric, expected) in [
            (DistanceMetric::Cosine, "cosine"),
            (DistanceMetric::L2, "l2"),
            (DistanceMetric::Ip, "ip"),
        ] {
            let chroma = MockChroma::start(MockChromaConfig::empty()).await;

            let guard = lock_chroma_endpoint();
            let app = test::init_service(
                App::new()
                    .app_data(web::Data::new(chroma.base_url.clone()))
                    .service(create_collection),
            )
            .await;
            let resp = test::call_service(
                &app,
                post(CreateCollectionRequest {
                    name: "notes".to_string(),
                    metadata: None,
                    distance_metric: Some(metric),
                    embedding_model: None,
                })
                .to_request(),
            )
            .await;
            assert_eq!(resp.status().as_u16(), 200);
            drop(guard);

            assert_eq!(
                chroma.requests()[0].body.as_ref().unwrap()["metadata"]["hnsw:space"],
                expected
            );

            chroma.stop().await;
        }
    }

    #[actix_web::test]
    async fn test_create_collection_reports_a_name_clash_as_500() {
        let chroma = MockChroma::start(MockChromaConfig::holding(vec![MockChromaCollection::new(
            "notes",
        )]))
        .await;

        let guard = lock_chroma_endpoint();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(chroma.base_url.clone()))
                .service(create_collection),
        )
        .await;
        let resp = test::call_service(&app, post(named("notes")).to_request()).await;

        assert_eq!(resp.status().as_u16(), 500);
        let body: ChromaDBResponse<Collection> = test::read_body_json(resp).await;
        drop(guard);

        assert!(!body.success);
        assert!(body.data.is_none());
        assert!(body
            .error
            .unwrap()
            .contains("Failed to create collection: Failed to create collection 'notes'"));

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_create_collection_reports_an_unusable_address_as_503() {
        let _guard = lock_chroma_endpoint();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UNPARSEABLE_CHROMA_ENDPOINT.to_string()))
                .service(create_collection),
        )
        .await;
        let resp = test::call_service(&app, post(named("notes")).to_request()).await;

        assert_eq!(resp.status().as_u16(), 503);
        let body: ChromaDBResponse<Collection> = test::read_body_json(resp).await;
        assert!(!body.success);
        assert!(body
            .error
            .unwrap()
            .contains("Failed to connect to ChromaDB"));
    }
}
