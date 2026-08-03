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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::sync::mpsc::UnboundedReceiver;

    // `ws_handler` itself is only an `actix_ws::handle` upgrade plus two spawned
    // pumps, so the tests below cover the session bookkeeping it delegates to.

    fn drain(rx: &mut UnboundedReceiver<String>) -> Vec<Value> {
        let mut out = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            out.push(serde_json::from_str(&msg).unwrap());
        }
        out
    }

    #[test]
    fn test_new_and_default_start_with_no_clients() {
        assert!(ChromaWebSocketState::new()
            .clients
            .lock()
            .unwrap()
            .is_empty());
        assert!(ChromaWebSocketState::default()
            .clients
            .lock()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn test_clients_are_registered_and_removed_by_id() {
        let state = ChromaWebSocketState::new();
        let (tx_a, _rx_a) = mpsc::unbounded_channel();
        let (tx_b, _rx_b) = mpsc::unbounded_channel();

        state.add_client("a".to_string(), tx_a);
        state.add_client("b".to_string(), tx_b);
        assert_eq!(state.clients.lock().unwrap().len(), 2);

        state.remove_client("a");
        let clients = state.clients.lock().unwrap();
        assert_eq!(clients.len(), 1);
        assert!(clients.contains_key("b"));
    }

    #[test]
    fn test_re_adding_the_same_id_replaces_the_sender() {
        let state = ChromaWebSocketState::new();
        let (tx_first, mut rx_first) = mpsc::unbounded_channel();
        let (tx_second, mut rx_second) = mpsc::unbounded_channel();

        state.add_client("a".to_string(), tx_first);
        state.add_client("a".to_string(), tx_second);

        state.broadcast(json!({"type": "upload_progress"}));

        assert_eq!(state.clients.lock().unwrap().len(), 1);
        assert!(drain(&mut rx_first).is_empty());
        assert_eq!(drain(&mut rx_second).len(), 1);
    }

    #[test]
    fn test_removing_an_unknown_client_is_a_no_op() {
        let state = ChromaWebSocketState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        state.add_client("a".to_string(), tx);

        state.remove_client("nope");

        assert_eq!(state.clients.lock().unwrap().len(), 1);
    }

    #[test]
    fn test_clones_share_the_same_client_registry() {
        let state = ChromaWebSocketState::new();
        let clone = state.clone();
        let (tx, mut rx) = mpsc::unbounded_channel();

        clone.add_client("a".to_string(), tx);
        state.broadcast(json!({"type": "upload_progress", "percent": 50}));

        let msgs = drain(&mut rx);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["percent"], 50);
    }

    #[test]
    fn test_broadcast_reaches_every_client_with_the_serialised_payload() {
        let state = ChromaWebSocketState::new();
        let (tx_a, mut rx_a) = mpsc::unbounded_channel();
        let (tx_b, mut rx_b) = mpsc::unbounded_channel();
        state.add_client("a".to_string(), tx_a);
        state.add_client("b".to_string(), tx_b);

        state.broadcast(json!({
            "type": "upload_progress",
            "collection": "notes",
            "stage": "embedding",
        }));

        for rx in [&mut rx_a, &mut rx_b] {
            let msgs = drain(rx);
            assert_eq!(msgs.len(), 1);
            assert_eq!(msgs[0]["type"], "upload_progress");
            assert_eq!(msgs[0]["collection"], "notes");
            assert_eq!(msgs[0]["stage"], "embedding");
        }
    }

    #[test]
    fn test_broadcast_tolerates_a_dropped_receiver() {
        let state = ChromaWebSocketState::new();
        let (tx_dead, rx_dead) = mpsc::unbounded_channel();
        let (tx_live, mut rx_live) = mpsc::unbounded_channel();
        state.add_client("dead".to_string(), tx_dead);
        state.add_client("live".to_string(), tx_live);
        drop(rx_dead);

        state.broadcast(json!({"type": "done"}));

        assert_eq!(drain(&mut rx_live).len(), 1);
        // The dead client is reported but not evicted.
        assert_eq!(state.clients.lock().unwrap().len(), 2);
    }

    #[test]
    fn test_broadcast_with_no_clients_is_harmless() {
        let state = ChromaWebSocketState::new();

        state.broadcast(json!({"type": "done"}));

        assert!(state.clients.lock().unwrap().is_empty());
    }

    #[test]
    fn test_broadcast_preserves_every_message_in_order() {
        let state = ChromaWebSocketState::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        state.add_client("a".to_string(), tx);

        for percent in [0, 50, 100] {
            state.broadcast(json!({"percent": percent}));
        }

        let msgs = drain(&mut rx);
        let percents: Vec<i64> = msgs
            .iter()
            .map(|msg| msg["percent"].as_i64().unwrap())
            .collect();
        assert_eq!(percents, vec![0, 50, 100]);
    }
}
