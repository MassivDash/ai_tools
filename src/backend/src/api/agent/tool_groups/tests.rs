use super::sqlite_storage::ToolGroupsStorage;
use sqlx::sqlite::SqlitePoolOptions;

async fn setup_storage() -> ToolGroupsStorage {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");

    ToolGroupsStorage::new(pool)
        .await
        .expect("Failed to initialize storage")
}

#[tokio::test]
async fn test_create_and_get_groups() {
    let storage = setup_storage().await;

    let group = storage
        .create_group(
            "post writer",
            &["bluesky_post".to_string(), "facebook_post".to_string()],
        )
        .await
        .expect("Failed to create group");

    assert_eq!(group.name, "post writer");
    assert_eq!(group.tool_types, vec!["bluesky_post", "facebook_post"]);

    let groups = storage
        .get_all_groups()
        .await
        .expect("Failed to get groups");
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].id, group.id);
}

#[tokio::test]
async fn test_update_group_renames_and_replaces_tools() {
    let storage = setup_storage().await;

    let group = storage
        .create_group("original", &["bluesky_post".to_string()])
        .await
        .expect("Failed to create group");

    let updated = storage
        .update_group(
            group.id,
            "renamed",
            &["google_tasks_write".to_string(), "google_gmail".to_string()],
        )
        .await
        .expect("Failed to update group")
        .expect("Expected updated group");

    assert_eq!(updated.name, "renamed");
    assert_eq!(
        updated.tool_types,
        vec!["google_tasks_write", "google_gmail"]
    );
}

#[tokio::test]
async fn test_update_nonexistent_group_returns_none() {
    let storage = setup_storage().await;

    let result = storage
        .update_group(999, "does not exist", &["bluesky_post".to_string()])
        .await
        .expect("Update should not error");

    assert!(result.is_none());
}

#[tokio::test]
async fn test_delete_group() {
    let storage = setup_storage().await;

    let group = storage
        .create_group("to delete", &["bluesky_post".to_string()])
        .await
        .expect("Failed to create group");

    let deleted = storage
        .delete_group(group.id)
        .await
        .expect("Delete should not error");
    assert!(deleted);

    let groups = storage
        .get_all_groups()
        .await
        .expect("Failed to get groups");
    assert!(groups.is_empty());
}

#[tokio::test]
async fn test_delete_nonexistent_group_returns_false() {
    let storage = setup_storage().await;

    let deleted = storage
        .delete_group(999)
        .await
        .expect("Delete should not error");
    assert!(!deleted);
}

#[tokio::test]
async fn test_duplicate_name_on_create_returns_error() {
    let storage = setup_storage().await;

    storage
        .create_group("post writer", &["bluesky_post".to_string()])
        .await
        .expect("Failed to create group");

    let result = storage
        .create_group("post writer", &["facebook_post".to_string()])
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_group_by_id() {
    let storage = setup_storage().await;

    let group = storage
        .create_group("readers", &["google_gmail_read".to_string()])
        .await
        .expect("Failed to create group");

    let found = storage
        .get_group(group.id)
        .await
        .expect("Failed to get group")
        .expect("Expected the group to exist");
    assert_eq!(found.name, "readers");
    assert_eq!(found.tool_types, vec!["google_gmail_read"]);

    assert!(storage
        .get_group(999)
        .await
        .expect("Failed to get group")
        .is_none());
}

#[tokio::test]
async fn test_get_all_groups_is_ordered_by_name() {
    let storage = setup_storage().await;

    for name in ["zulu", "alpha", "mike"] {
        storage
            .create_group(name, &["bluesky_post".to_string()])
            .await
            .expect("Failed to create group");
    }

    let names: Vec<String> = storage
        .get_all_groups()
        .await
        .expect("Failed to get groups")
        .into_iter()
        .map(|g| g.name)
        .collect();
    assert_eq!(names, vec!["alpha", "mike", "zulu"]);
}

mod handlers {
    use super::setup_storage;
    use crate::api::agent::tool_groups::sqlite_storage::ToolGroupsStorage;
    use crate::api::agent::tool_groups::types::{ToolGroupResponse, ToolGroupsResponse};
    use crate::api::agent::tool_groups::{
        create_tool_group, delete_tool_group, get_tool_groups, update_tool_group,
    };
    use actix_web::{test, web, App};

    macro_rules! tool_groups_app {
        ($storage:expr) => {
            test::init_service(
                App::new()
                    .app_data(web::Data::new($storage))
                    .service(get_tool_groups)
                    .service(create_tool_group)
                    .service(update_tool_group)
                    .service(delete_tool_group),
            )
            .await
        };
    }

    async fn broken_storage() -> ToolGroupsStorage {
        let storage = setup_storage().await;
        storage.drop_table_for_tests().await;
        storage
    }

    #[actix_web::test]
    async fn test_get_tool_groups_returns_the_stored_groups() {
        let storage = setup_storage().await;
        let group = storage
            .create_group("post writer", &["bluesky_post".to_string()])
            .await
            .expect("Failed to create group");
        let app = tool_groups_app!(storage);

        let req = test::TestRequest::get()
            .uri("/api/agent/tool-groups")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: ToolGroupsResponse = test::read_body_json(resp).await;
        assert_eq!(body.groups.len(), 1);
        assert_eq!(body.groups[0].id, group.id);
        assert_eq!(body.groups[0].name, "post writer");
        assert_eq!(body.groups[0].tool_types, vec!["bluesky_post"]);
    }

    #[actix_web::test]
    async fn test_get_tool_groups_reports_storage_errors() {
        let app = tool_groups_app!(broken_storage().await);

        let req = test::TestRequest::get()
            .uri("/api/agent/tool-groups")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 500);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(
            body["error"]
                .as_str()
                .unwrap_or_default()
                .starts_with("Failed to fetch tool groups"),
            "unexpected body: {}",
            body
        );
    }

    #[actix_web::test]
    async fn test_create_tool_group_persists_the_group() {
        let storage = setup_storage().await;
        let app = tool_groups_app!(storage.clone());

        let req = test::TestRequest::post()
            .uri("/api/agent/tool-groups")
            .set_json(serde_json::json!({
                "name": "  post writer  ",
                "tool_types": ["bluesky_post", "facebook_post"]
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: ToolGroupResponse = test::read_body_json(resp).await;
        assert_eq!(body.group.name, "post writer", "the name is trimmed");
        assert_eq!(body.group.tool_types, vec!["bluesky_post", "facebook_post"]);

        let stored = storage
            .get_all_groups()
            .await
            .expect("Failed to get groups");
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].name, "post writer");
    }

    #[actix_web::test]
    async fn test_create_tool_group_rejects_empty_name_or_tools() {
        let storage = setup_storage().await;
        let app = tool_groups_app!(storage.clone());

        for body in [
            serde_json::json!({ "name": "   ", "tool_types": ["bluesky_post"] }),
            serde_json::json!({ "name": "valid", "tool_types": [] }),
        ] {
            let req = test::TestRequest::post()
                .uri("/api/agent/tool-groups")
                .set_json(&body)
                .to_request();
            let resp = test::call_service(&app, req).await;

            assert_eq!(resp.status().as_u16(), 400, "should reject {}", body);
            let error: serde_json::Value = test::read_body_json(resp).await;
            assert_eq!(
                error["error"],
                "A group name and at least one tool are required"
            );
        }

        assert!(storage
            .get_all_groups()
            .await
            .expect("Failed to get groups")
            .is_empty());
    }

    /// The handler means to answer a duplicate name with 409, but the storage
    /// layer wraps the sqlx error in `anyhow::Context`, so `e.to_string()` is
    /// only the context line ("Failed to create tool group: ...") and never
    /// contains "UNIQUE constraint". This pins the behaviour that is actually
    /// observable today: a 500 carrying the storage error.
    #[actix_web::test]
    async fn test_create_tool_group_with_a_duplicate_name_falls_through_to_500() {
        let storage = setup_storage().await;
        storage
            .create_group("post writer", &["bluesky_post".to_string()])
            .await
            .expect("Failed to create group");
        let app = tool_groups_app!(storage);

        let req = test::TestRequest::post()
            .uri("/api/agent/tool-groups")
            .set_json(serde_json::json!({
                "name": "post writer",
                "tool_types": ["facebook_post"]
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(
            resp.status().as_u16(),
            500,
            "the 409 branch is unreachable while the storage error is wrapped in context"
        );
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(
            body["error"],
            "Failed to create tool group: Failed to create tool group: post writer"
        );
    }

    #[actix_web::test]
    async fn test_create_tool_group_reports_other_storage_errors() {
        let app = tool_groups_app!(broken_storage().await);

        let req = test::TestRequest::post()
            .uri("/api/agent/tool-groups")
            .set_json(serde_json::json!({
                "name": "post writer",
                "tool_types": ["bluesky_post"]
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 500);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(
            body["error"]
                .as_str()
                .unwrap_or_default()
                .starts_with("Failed to create tool group"),
            "unexpected body: {}",
            body
        );
    }

    #[actix_web::test]
    async fn test_update_tool_group_persists_the_changes() {
        let storage = setup_storage().await;
        let group = storage
            .create_group("original", &["bluesky_post".to_string()])
            .await
            .expect("Failed to create group");
        let app = tool_groups_app!(storage.clone());

        let req = test::TestRequest::put()
            .uri(&format!("/api/agent/tool-groups/{}", group.id))
            .set_json(serde_json::json!({
                "name": " renamed ",
                "tool_types": ["google_gmail"]
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: ToolGroupResponse = test::read_body_json(resp).await;
        assert_eq!(body.group.id, group.id);
        assert_eq!(body.group.name, "renamed");
        assert_eq!(body.group.tool_types, vec!["google_gmail"]);

        let stored = storage
            .get_group(group.id)
            .await
            .expect("Failed to get group")
            .expect("Expected the group to exist");
        assert_eq!(stored.name, "renamed");
    }

    #[actix_web::test]
    async fn test_update_tool_group_rejects_empty_name_or_tools() {
        let storage = setup_storage().await;
        let group = storage
            .create_group("original", &["bluesky_post".to_string()])
            .await
            .expect("Failed to create group");
        let app = tool_groups_app!(storage.clone());

        for body in [
            serde_json::json!({ "name": "", "tool_types": ["bluesky_post"] }),
            serde_json::json!({ "name": "valid", "tool_types": [] }),
        ] {
            let req = test::TestRequest::put()
                .uri(&format!("/api/agent/tool-groups/{}", group.id))
                .set_json(&body)
                .to_request();
            let resp = test::call_service(&app, req).await;

            assert_eq!(resp.status().as_u16(), 400, "should reject {}", body);
        }

        assert_eq!(
            storage
                .get_group(group.id)
                .await
                .expect("Failed to get group")
                .expect("Expected the group to exist")
                .name,
            "original"
        );
    }

    #[actix_web::test]
    async fn test_update_tool_group_returns_not_found_for_an_unknown_id() {
        let storage = setup_storage().await;
        let app = tool_groups_app!(storage);

        let req = test::TestRequest::put()
            .uri("/api/agent/tool-groups/999")
            .set_json(serde_json::json!({
                "name": "renamed",
                "tool_types": ["bluesky_post"]
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 404);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Tool group not found");
    }

    /// Same unreachable-409 story as on create: the duplicate name surfaces as
    /// a 500 because the UNIQUE violation is hidden behind an anyhow context.
    #[actix_web::test]
    async fn test_update_tool_group_with_a_duplicate_name_falls_through_to_500() {
        let storage = setup_storage().await;
        storage
            .create_group("taken", &["bluesky_post".to_string()])
            .await
            .expect("Failed to create group");
        let group = storage
            .create_group("original", &["bluesky_post".to_string()])
            .await
            .expect("Failed to create group");
        let app = tool_groups_app!(storage.clone());

        let req = test::TestRequest::put()
            .uri(&format!("/api/agent/tool-groups/{}", group.id))
            .set_json(serde_json::json!({
                "name": "taken",
                "tool_types": ["bluesky_post"]
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(
            resp.status().as_u16(),
            500,
            "the 409 branch is unreachable while the storage error is wrapped in context"
        );
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(
            body["error"]
                .as_str()
                .unwrap_or_default()
                .starts_with("Failed to update tool group"),
            "unexpected body: {}",
            body
        );

        // The stored group keeps its old name
        let unchanged = storage
            .get_group(group.id)
            .await
            .expect("Failed to get group")
            .expect("Expected the group to exist");
        assert_eq!(unchanged.name, "original");
    }

    #[actix_web::test]
    async fn test_update_tool_group_reports_other_storage_errors() {
        let app = tool_groups_app!(broken_storage().await);

        let req = test::TestRequest::put()
            .uri("/api/agent/tool-groups/1")
            .set_json(serde_json::json!({
                "name": "renamed",
                "tool_types": ["bluesky_post"]
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 500);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(
            body["error"]
                .as_str()
                .unwrap_or_default()
                .starts_with("Failed to update tool group"),
            "unexpected body: {}",
            body
        );
    }

    #[actix_web::test]
    async fn test_delete_tool_group_removes_it() {
        let storage = setup_storage().await;
        let group = storage
            .create_group("doomed", &["bluesky_post".to_string()])
            .await
            .expect("Failed to create group");
        let app = tool_groups_app!(storage.clone());

        let req = test::TestRequest::delete()
            .uri(&format!("/api/agent/tool-groups/{}", group.id))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body, serde_json::json!({ "success": true }));
        assert!(storage
            .get_all_groups()
            .await
            .expect("Failed to get groups")
            .is_empty());
    }

    #[actix_web::test]
    async fn test_delete_tool_group_returns_not_found_for_an_unknown_id() {
        let storage = setup_storage().await;
        let app = tool_groups_app!(storage);

        let req = test::TestRequest::delete()
            .uri("/api/agent/tool-groups/999")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 404);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Tool group not found");
    }

    #[actix_web::test]
    async fn test_delete_tool_group_reports_storage_errors() {
        let app = tool_groups_app!(broken_storage().await);

        let req = test::TestRequest::delete()
            .uri("/api/agent/tool-groups/1")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 500);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(
            body["error"]
                .as_str()
                .unwrap_or_default()
                .starts_with("Failed to delete tool group"),
            "unexpected body: {}",
            body
        );
    }

    #[actix_web::test]
    async fn test_tool_group_routes_reject_a_non_numeric_id() {
        let storage = setup_storage().await;
        let app = tool_groups_app!(storage);

        let req = test::TestRequest::delete()
            .uri("/api/agent/tool-groups/not-a-number")
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status().as_u16(), 404);
    }
}
