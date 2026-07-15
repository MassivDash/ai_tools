use actix_web::{web, web::Payload, Error, HttpRequest, HttpResponse};
use actix_ws::Message;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ChromaWebSocketState {
    pub clients: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>,
}

impl ChromaWebSocketState {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_client(&self, client_id: String, tx: mpsc::UnboundedSender<String>) {
        let mut clients = self.clients.lock().unwrap();
        clients.insert(client_id, tx);
        println!("📡 ChromaDB WebSocket client connected");
    }

    pub fn remove_client(&self, client_id: &str) {
        let mut clients = self.clients.lock().unwrap();
        clients.remove(client_id);
        println!("📡 ChromaDB WebSocket client disconnected");
    }

    pub fn broadcast(&self, message: Value) {
        let clients = self.clients.lock().unwrap();
        let msg_str = serde_json::to_string(&message).unwrap_or_else(|_| "{}".to_string());

        for (client_id, tx) in clients.iter() {
            if tx.send(msg_str.clone()).is_err() {
                println!("⚠️ Failed to send to ChromaDB WS client {}", client_id);
            }
        }
    }
}

impl Default for ChromaWebSocketState {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn ws_handler(
    req: HttpRequest,
    body: Payload,
    state: web::Data<ChromaWebSocketState>,
) -> Result<HttpResponse, Error> {
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;
    let client_id = Uuid::new_v4().to_string();

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    state.add_client(client_id.clone(), tx);

    let state_clone = state.clone();
    let client_id_clone = client_id.clone();

    // Spawn a task to send messages from the channel to the WebSocket
    actix_web::rt::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if session.text(msg).await.is_err() {
                break;
            }
        }
        state_clone.remove_client(&client_id_clone);
        let _ = session.close(None).await;
    });

    // Spawn a task to read messages from the WebSocket (to handle Ping/Close)
    let state_clone2 = state.clone();
    actix_web::rt::spawn(async move {
        while let Some(Ok(msg)) = msg_stream.recv().await {
            if let Message::Close(_) = msg {
                break;
            }
        }
        state_clone2.remove_client(&client_id);
    });

    Ok(response)
}
