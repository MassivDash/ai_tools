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
