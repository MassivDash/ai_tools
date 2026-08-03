use super::storage::TestingStorage;
use sqlx::sqlite::SqlitePoolOptions;

async fn setup_storage() -> TestingStorage {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");

    TestingStorage::new(pool)
        .await
        .expect("Failed to initialize storage")
}

#[tokio::test]
async fn test_create_and_get_suites() {
    let storage = setup_storage().await;

    // Create a suite
    let suite = storage
        .create_suite("Test Suite 1".to_string(), Some("Description".to_string()))
        .await
        .expect("Failed to create suite");

    assert_eq!(suite.name, "Test Suite 1");
    assert_eq!(suite.description, Some("Description".to_string()));

    // Get suites
    let suites = storage.get_suites().await.expect("Failed to get suites");
    assert_eq!(suites.len(), 1);
    assert_eq!(suites[0].id, suite.id);
}

#[tokio::test]
async fn test_update_and_delete_suite() {
    let storage = setup_storage().await;

    let suite = storage
        .create_suite("Original Name".to_string(), None)
        .await
        .expect("Failed to create suite");

    // Update
    storage
        .update_suite(
            &suite.id,
            "Updated Name".to_string(),
            Some("New Desc".to_string()),
        )
        .await
        .expect("Failed to update suite");

    let suites = storage.get_suites().await.expect("Failed to get suites");
    assert_eq!(suites[0].name, "Updated Name");
    assert_eq!(suites[0].description, Some("New Desc".to_string()));

    // Delete
    storage
        .delete_suite(&suite.id)
        .await
        .expect("Failed to delete suite");

    let suites = storage.get_suites().await.expect("Failed to get suites");
    assert!(suites.is_empty());
}

#[tokio::test]
async fn test_questions_crud() {
    let storage = setup_storage().await;

    let suite = storage
        .create_suite("Questions Suite".to_string(), None)
        .await
        .expect("Failed to create suite");

    // Add question
    let q1 = storage
        .add_question(&suite.id, "Question 1".to_string())
        .await
        .expect("Failed to add question");

    assert_eq!(q1.content, "Question 1");
    assert_eq!(q1.suite_id, suite.id);

    // Get questions
    let questions = storage
        .get_questions(&suite.id)
        .await
        .expect("Failed to get questions");
    assert_eq!(questions.len(), 1);
    assert_eq!(questions[0].id, q1.id);

    // Update question
    storage
        .update_question(q1.id, "Question 1 Updated".to_string())
        .await
        .expect("Failed to update question");

    let questions = storage
        .get_questions(&suite.id)
        .await
        .expect("Failed to get questions");
    assert_eq!(questions[0].content, "Question 1 Updated");

    // Delete question
    storage
        .delete_question(q1.id)
        .await
        .expect("Failed to delete question");

    let questions = storage
        .get_questions(&suite.id)
        .await
        .expect("Failed to get questions");
    assert!(questions.is_empty());
}

#[tokio::test]
async fn test_cascade_delete() {
    let storage = setup_storage().await;

    let suite = storage
        .create_suite("Cascade Suite".to_string(), None)
        .await
        .expect("Failed to create suite");

    storage
        .add_question(&suite.id, "Q1".to_string())
        .await
        .expect("Failed to add question");

    // Delete suite
    storage
        .delete_suite(&suite.id)
        .await
        .expect("Failed to delete suite");

    // `initialize` turns foreign keys on for the pooled connection, so the
    // questions must go away with their suite.
    let questions = storage
        .get_questions(&suite.id)
        .await
        .expect("Failed to get questions");
    assert!(
        questions.is_empty(),
        "deleting a suite should cascade to its questions, found {:?}",
        questions
    );
}

#[tokio::test]
async fn test_get_questions_only_returns_the_requested_suite() {
    let storage = setup_storage().await;

    let first = storage
        .create_suite("First".to_string(), None)
        .await
        .expect("Failed to create suite");
    let second = storage
        .create_suite("Second".to_string(), None)
        .await
        .expect("Failed to create suite");

    storage
        .add_question(&first.id, "belongs to first".to_string())
        .await
        .expect("Failed to add question");
    storage
        .add_question(&second.id, "belongs to second".to_string())
        .await
        .expect("Failed to add question");

    let questions = storage
        .get_questions(&first.id)
        .await
        .expect("Failed to get questions");
    assert_eq!(questions.len(), 1);
    assert_eq!(questions[0].content, "belongs to first");
}

mod routes {
    use super::setup_storage;
    use crate::api::agent::testing::routes::*;
    use crate::api::agent::testing::storage::{TestQuestion, TestSuite, TestingStorage};
    use actix_web::{test, web, App};

    /// Mounts every testing route under the same `/api/agent/testing` scope the
    /// real service configuration uses.
    macro_rules! testing_app {
        ($storage:expr) => {
            test::init_service(
                App::new().app_data(web::Data::new($storage)).service(
                    web::scope("/api/agent/testing")
                        .service(get_suites)
                        .service(create_suite)
                        .service(update_suite)
                        .service(delete_suite)
                        .service(get_questions)
                        .service(add_question)
                        .service(update_question)
                        .service(delete_question),
                ),
            )
            .await
        };
    }

    async fn broken_storage() -> TestingStorage {
        let storage = setup_storage().await;
        storage.drop_tables_for_tests().await;
        storage
    }

    #[actix_web::test]
    async fn test_get_suites_returns_the_stored_suites() {
        let storage = setup_storage().await;
        let created = storage
            .create_suite("Suite A".to_string(), Some("desc".to_string()))
            .await
            .expect("Failed to create suite");

        let app = testing_app!(storage);

        let req = test::TestRequest::get()
            .uri("/api/agent/testing/suites")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: Vec<TestSuite> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].id, created.id);
        assert_eq!(body[0].name, "Suite A");
        assert_eq!(body[0].description.as_deref(), Some("desc"));
    }

    #[actix_web::test]
    async fn test_create_suite_persists_and_echoes_the_suite() {
        let storage = setup_storage().await;
        let app = testing_app!(storage.clone());

        let req = test::TestRequest::post()
            .uri("/api/agent/testing/suites")
            .set_json(serde_json::json!({ "name": "New Suite" }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: TestSuite = test::read_body_json(resp).await;
        assert_eq!(body.name, "New Suite");
        assert!(body.description.is_none());
        assert!(body.created_at > 0);

        let stored = storage.get_suites().await.expect("Failed to get suites");
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].id, body.id);
    }

    #[actix_web::test]
    async fn test_create_suite_rejects_a_body_without_a_name() {
        let storage = setup_storage().await;
        let app = testing_app!(storage);

        let req = test::TestRequest::post()
            .uri("/api/agent/testing/suites")
            .set_json(serde_json::json!({ "description": "no name" }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 400);
    }

    #[actix_web::test]
    async fn test_update_suite_persists_the_changes() {
        let storage = setup_storage().await;
        let suite = storage
            .create_suite("Before".to_string(), None)
            .await
            .expect("Failed to create suite");
        let app = testing_app!(storage.clone());

        let req = test::TestRequest::put()
            .uri(&format!("/api/agent/testing/suites/{}", suite.id))
            .set_json(serde_json::json!({ "name": "After", "description": "now described" }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body, serde_json::json!({ "success": true }));

        let stored = storage.get_suites().await.expect("Failed to get suites");
        assert_eq!(stored[0].name, "After");
        assert_eq!(stored[0].description.as_deref(), Some("now described"));
    }

    #[actix_web::test]
    async fn test_delete_suite_removes_it() {
        let storage = setup_storage().await;
        let suite = storage
            .create_suite("Doomed".to_string(), None)
            .await
            .expect("Failed to create suite");
        let app = testing_app!(storage.clone());

        let req = test::TestRequest::delete()
            .uri(&format!("/api/agent/testing/suites/{}", suite.id))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        assert!(storage
            .get_suites()
            .await
            .expect("Failed to get suites")
            .is_empty());
    }

    #[actix_web::test]
    async fn test_question_routes_round_trip() {
        let storage = setup_storage().await;
        let suite = storage
            .create_suite("Questions".to_string(), None)
            .await
            .expect("Failed to create suite");
        let app = testing_app!(storage.clone());

        // Add
        let req = test::TestRequest::post()
            .uri(&format!("/api/agent/testing/suites/{}/questions", suite.id))
            .set_json(serde_json::json!({ "content": "What is 2 + 2?" }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 200);
        let created: TestQuestion = test::read_body_json(resp).await;
        assert_eq!(created.content, "What is 2 + 2?");
        assert_eq!(created.suite_id, suite.id);

        // List
        let req = test::TestRequest::get()
            .uri(&format!("/api/agent/testing/suites/{}/questions", suite.id))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 200);
        let listed: Vec<TestQuestion> = test::read_body_json(resp).await;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, created.id);

        // Update
        let req = test::TestRequest::put()
            .uri(&format!("/api/agent/testing/questions/{}", created.id))
            .set_json(serde_json::json!({ "content": "What is 3 + 3?" }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 200);
        assert_eq!(
            storage
                .get_questions(&suite.id)
                .await
                .expect("Failed to get questions")[0]
                .content,
            "What is 3 + 3?"
        );

        // Delete
        let req = test::TestRequest::delete()
            .uri(&format!("/api/agent/testing/questions/{}", created.id))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 200);
        assert!(storage
            .get_questions(&suite.id)
            .await
            .expect("Failed to get questions")
            .is_empty());
    }

    #[actix_web::test]
    async fn test_question_routes_reject_a_non_numeric_id() {
        let storage = setup_storage().await;
        let app = testing_app!(storage);

        for method in ["PUT", "DELETE"] {
            let req = match method {
                "PUT" => test::TestRequest::put()
                    .uri("/api/agent/testing/questions/not-a-number")
                    .set_json(serde_json::json!({ "content": "x" })),
                _ => test::TestRequest::delete().uri("/api/agent/testing/questions/not-a-number"),
            };
            let resp = test::call_service(&app, req.to_request()).await;
            assert_eq!(
                resp.status().as_u16(),
                404,
                "{} with a non-numeric id should not match the route",
                method
            );
        }
    }

    #[actix_web::test]
    async fn test_every_route_reports_storage_errors() {
        let app = testing_app!(broken_storage().await);

        let requests = vec![
            test::TestRequest::get()
                .uri("/api/agent/testing/suites")
                .to_request(),
            test::TestRequest::post()
                .uri("/api/agent/testing/suites")
                .set_json(serde_json::json!({ "name": "x" }))
                .to_request(),
            test::TestRequest::put()
                .uri("/api/agent/testing/suites/some-id")
                .set_json(serde_json::json!({ "name": "x" }))
                .to_request(),
            test::TestRequest::delete()
                .uri("/api/agent/testing/suites/some-id")
                .to_request(),
            test::TestRequest::get()
                .uri("/api/agent/testing/suites/some-id/questions")
                .to_request(),
            test::TestRequest::post()
                .uri("/api/agent/testing/suites/some-id/questions")
                .set_json(serde_json::json!({ "content": "x" }))
                .to_request(),
            test::TestRequest::put()
                .uri("/api/agent/testing/questions/1")
                .set_json(serde_json::json!({ "content": "x" }))
                .to_request(),
            test::TestRequest::delete()
                .uri("/api/agent/testing/questions/1")
                .to_request(),
        ];

        for req in requests {
            let method = req.method().clone();
            let uri = req.uri().clone();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status().as_u16(),
                500,
                "{} {} should report the storage failure",
                method,
                uri
            );
            let body: serde_json::Value = test::read_body_json(resp).await;
            assert!(
                body.get("error")
                    .and_then(|e| e.as_str())
                    .is_some_and(|e| !e.is_empty()),
                "expected an error message, got {}",
                body
            );
        }
    }
}
