use crate::api::agent::core::types::UpdateConversationRequest;
use crate::api::agent::memory::sqlite_memory::SqliteConversationMemory;
use actix_web::{delete, get, patch, web, HttpResponse, Result as ActixResult};
use std::sync::Arc;

/// Get all conversations
#[get("/api/agent/conversations")]
pub async fn get_conversations(
    sqlite_memory: web::Data<Arc<SqliteConversationMemory>>,
) -> ActixResult<HttpResponse> {
    match sqlite_memory.get_conversations().await {
        Ok(conversations) => Ok(HttpResponse::Ok().json(conversations)),
        Err(e) => {
            println!("Failed to fetch conversations: {}", e);
            Ok(HttpResponse::InternalServerError()
                .body(format!("Failed to fetch conversations: {}", e)))
        }
    }
}

/// Delete a conversation
#[delete("/api/agent/conversations/{id}")]
pub async fn delete_conversation(
    path: web::Path<String>,
    sqlite_memory: web::Data<Arc<SqliteConversationMemory>>,
) -> ActixResult<HttpResponse> {
    let conversation_id = path.into_inner();

    match sqlite_memory.delete_conversation(&conversation_id).await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(e) => {
            println!("Failed to delete conversation {}: {}", conversation_id, e);
            Ok(HttpResponse::InternalServerError()
                .body(format!("Failed to delete conversation: {}", e)))
        }
    }
}

/// Update conversation title
#[patch("/api/agent/conversations/{id}")]
pub async fn update_conversation_title(
    path: web::Path<String>,
    body: web::Json<UpdateConversationRequest>,
    sqlite_memory: web::Data<Arc<SqliteConversationMemory>>,
) -> ActixResult<HttpResponse> {
    let conversation_id = path.into_inner();

    match sqlite_memory
        .update_conversation_title(&conversation_id, &body.title)
        .await
    {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(e) => {
            println!(
                "Failed to update conversation {} title: {}",
                conversation_id, e
            );
            Ok(HttpResponse::InternalServerError()
                .body(format!("Failed to update conversation title: {}", e)))
        }
    }
}

/// Get conversation history
#[get("/api/agent/conversations/{id}/messages")]
pub async fn get_conversation_history(
    path: web::Path<String>,
    sqlite_memory: web::Data<Arc<SqliteConversationMemory>>,
) -> ActixResult<HttpResponse> {
    let conversation_id = path.into_inner();

    match sqlite_memory.get_messages(&conversation_id).await {
        Ok(messages) => Ok(HttpResponse::Ok().json(messages)),
        Err(e) => {
            println!(
                "Failed to fetch messages for conversation {}: {}",
                conversation_id, e
            );
            Ok(
                HttpResponse::InternalServerError()
                    .body(format!("Failed to fetch messages: {}", e)),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::core::types::{ChatMessage, Conversation, MessageContent, MessageRole};
    use crate::api::agent::memory::sqlite_memory::new_test_memory;
    use actix_web::{test, App};
    use tempfile::TempDir;

    async fn setup() -> (TempDir, Arc<SqliteConversationMemory>) {
        let (dir, memory) = new_test_memory().await;
        (dir, Arc::new(memory))
    }

    fn user_message(text: &str) -> ChatMessage {
        ChatMessage {
            role: MessageRole::User,
            content: MessageContent::Text(text.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }
    }

    #[actix_web::test]
    async fn test_get_conversations_returns_the_stored_conversations() {
        let (_dir, memory) = setup().await;
        let id = memory
            .get_or_create_conversation_id(None, Some("model-a"))
            .await
            .expect("Failed to create conversation");

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(memory.clone()))
                .service(get_conversations),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/agent/conversations")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: Vec<Conversation> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].id, id);
        assert_eq!(body[0].model.as_deref(), Some("model-a"));
    }

    #[actix_web::test]
    async fn test_get_conversations_reports_storage_errors() {
        let (_dir, memory) = setup().await;
        memory.drop_tables_for_tests().await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(memory.clone()))
                .service(get_conversations),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/agent/conversations")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 500);
        let body = test::read_body(resp).await;
        assert!(
            String::from_utf8_lossy(&body).contains("Failed to fetch conversations"),
            "unexpected body: {:?}",
            body
        );
    }

    #[actix_web::test]
    async fn test_delete_conversation_removes_it() {
        let (_dir, memory) = setup().await;
        let id = memory
            .get_or_create_conversation_id(None, None)
            .await
            .expect("Failed to create conversation");

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(memory.clone()))
                .service(delete_conversation),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/agent/conversations/{}", id))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        assert!(memory
            .get_conversations()
            .await
            .expect("Failed to list conversations")
            .is_empty());
    }

    #[actix_web::test]
    async fn test_delete_conversation_reports_storage_errors() {
        let (_dir, memory) = setup().await;
        memory.drop_tables_for_tests().await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(memory.clone()))
                .service(delete_conversation),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri("/api/agent/conversations/whatever")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 500);
        let body = test::read_body(resp).await;
        assert!(
            String::from_utf8_lossy(&body).contains("Failed to delete conversation"),
            "unexpected body: {:?}",
            body
        );
    }

    #[actix_web::test]
    async fn test_update_conversation_title_persists_the_new_title() {
        let (_dir, memory) = setup().await;
        let id = memory
            .get_or_create_conversation_id(None, None)
            .await
            .expect("Failed to create conversation");

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(memory.clone()))
                .service(update_conversation_title),
        )
        .await;

        let req = test::TestRequest::patch()
            .uri(&format!("/api/agent/conversations/{}", id))
            .set_json(serde_json::json!({ "title": "Renamed chat" }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        assert_eq!(
            memory.get_title(&id).await.expect("Failed to get title"),
            "Renamed chat"
        );
    }

    #[actix_web::test]
    async fn test_update_conversation_title_rejects_a_malformed_body() {
        let (_dir, memory) = setup().await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(memory.clone()))
                .service(update_conversation_title),
        )
        .await;

        let req = test::TestRequest::patch()
            .uri("/api/agent/conversations/some-id")
            .set_json(serde_json::json!({ "not_a_title": 1 }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 400);
    }

    #[actix_web::test]
    async fn test_update_conversation_title_reports_storage_errors() {
        let (_dir, memory) = setup().await;
        memory.drop_tables_for_tests().await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(memory.clone()))
                .service(update_conversation_title),
        )
        .await;

        let req = test::TestRequest::patch()
            .uri("/api/agent/conversations/whatever")
            .set_json(serde_json::json!({ "title": "Renamed chat" }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 500);
        let body = test::read_body(resp).await;
        assert!(
            String::from_utf8_lossy(&body).contains("Failed to update conversation title"),
            "unexpected body: {:?}",
            body
        );
    }

    #[actix_web::test]
    async fn test_get_conversation_history_returns_the_messages_in_order() {
        let (_dir, memory) = setup().await;
        let id = memory
            .get_or_create_conversation_id(None, None)
            .await
            .expect("Failed to create conversation");
        for text in ["first", "second"] {
            memory
                .add_message(&id, user_message(text))
                .await
                .expect("Failed to add message");
        }

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(memory.clone()))
                .service(get_conversation_history),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/agent/conversations/{}/messages", id))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: Vec<ChatMessage> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 2);
        assert_eq!(body[0].content.text(), "first");
        assert_eq!(body[1].content.text(), "second");
        assert_eq!(body[0].role, MessageRole::User);
    }

    #[actix_web::test]
    async fn test_get_conversation_history_of_unknown_conversation_is_empty() {
        let (_dir, memory) = setup().await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(memory.clone()))
                .service(get_conversation_history),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/agent/conversations/nope/messages")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: Vec<ChatMessage> = test::read_body_json(resp).await;
        assert!(body.is_empty());
    }

    #[actix_web::test]
    async fn test_get_conversation_history_reports_storage_errors() {
        let (_dir, memory) = setup().await;
        memory.drop_tables_for_tests().await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(memory.clone()))
                .service(get_conversation_history),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/agent/conversations/whatever/messages")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 500);
        let body = test::read_body(resp).await;
        assert!(
            String::from_utf8_lossy(&body).contains("Failed to fetch messages"),
            "unexpected body: {:?}",
            body
        );
    }
}
