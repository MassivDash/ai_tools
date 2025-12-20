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
            println!("❌ Failed to fetch conversations: {}", e);
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
            println!(
                "❌ Failed to delete conversation {}: {}",
                conversation_id, e
            );
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
                "❌ Failed to update conversation {} title: {}",
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
                "❌ Failed to fetch messages for conversation {}: {}",
                conversation_id, e
            );
            Ok(
                HttpResponse::InternalServerError()
                    .body(format!("Failed to fetch messages: {}", e)),
            )
        }
    }
}
