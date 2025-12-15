use actix_web::{web, web::Payload, Error, HttpRequest, HttpResponse};
use actix_ws::{Message, Session};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::api::agent::types::AgentStreamEvent;

#[derive(Clone)]
pub struct AgentWebSocketState {
    pub clients: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>,
}

impl AgentWebSocketState {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_client(&self, client_id: String, tx: mpsc::UnboundedSender<String>) {
        let client_id_clone = client_id.clone();
        let mut clients = self.clients.lock().unwrap();
        clients.insert(client_id, tx);
        println!("📡 Agent WebSocket client connected: {}", client_id_clone);
    }

    pub fn remove_client(&self, client_id: &str) {
        let mut clients = self.clients.lock().unwrap();
        clients.remove(client_id);
        println!("📡 Agent WebSocket client disconnected: {}", client_id);
    }

    pub fn broadcast(&self, event: &AgentStreamEvent) {
        let clients = self.clients.lock().unwrap();
        // Send event directly as JSON (no wrapper needed, frontend handles AgentStreamEvent directly)
        let message = serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string());

        // Debug: log tool call events
        match event {
            AgentStreamEvent::ToolCall { tool_name, .. } => {
                println!("📡 Broadcasting ToolCall event for: {}", tool_name);
            }
            AgentStreamEvent::ToolResult { tool_name, .. } => {
                println!("📡 Broadcasting ToolResult event for: {}", tool_name);
            }
            _ => {}
        }

        for (client_id, tx) in clients.iter() {
            if tx.send(message.clone()).is_err() {
                println!("⚠️ Failed to send to client {}", client_id);
            }
        }
    }
}

impl Default for AgentWebSocketState {
    fn default() -> Self {
        Self::new()
    }
}

// WebSocket handler for agent streaming
pub async fn agent_ws(
    state: web::Data<Arc<AgentWebSocketState>>,
    session: Session,
    mut msg_stream: actix_ws::MessageStream,
) {
    let client_id = Uuid::new_v4().to_string();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Add client
    state.add_client(client_id.clone(), tx);

    // Clone session for sending messages
    let mut session_sender = session.clone();

    // Spawn task to send messages to client
    actix_rt::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if session_sender.text(msg).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    while let Some(Ok(msg)) = msg_stream.recv().await {
        match msg {
            Message::Text(_text) => {
                // Client can send messages (e.g., to request status)
                // For now, we just ignore client messages
            }
            Message::Close(_) => {
                break;
            }
            _ => {}
        }
    }

    // Remove client on disconnect
    state.remove_client(&client_id);
}

// HTTP handler for agent WebSocket
pub async fn agent_websocket(
    req: HttpRequest,
    stream: Payload,
    state: web::Data<Arc<AgentWebSocketState>>,
) -> Result<HttpResponse, Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;
    let state_clone = state.clone();
    actix_rt::spawn(agent_ws(state_clone, session, msg_stream));
    Ok(res)
}
