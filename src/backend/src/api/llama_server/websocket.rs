use actix_web::{web, Error, HttpRequest, HttpResponse, web::Payload};
use actix_ws::{Message, Session};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use crate::api::llama_server::types::{LogBuffer, LogSource, ProcessHandle, ServerStateHandle};

#[derive(Serialize, Debug, Clone)]
pub struct LogLine {
    pub timestamp: u64,
    pub line: String,
    pub source: String,
}

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    #[serde(rename = "log")]
    Log { log: LogLine },
    #[serde(rename = "status")]
    Status { active: bool, port: u16 },
    #[serde(rename = "logs_batch")]
    LogsBatch { logs: Vec<LogLine> },
}

#[derive(Clone)]
pub struct WebSocketState {
    pub logs_clients: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>,
    pub status_clients: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>,
    pub log_buffer: web::Data<LogBuffer>,
    pub process: web::Data<ProcessHandle>,
    pub server_state: web::Data<ServerStateHandle>,
}

impl WebSocketState {
    pub fn new(
        log_buffer: web::Data<LogBuffer>,
        process: web::Data<ProcessHandle>,
        server_state: web::Data<ServerStateHandle>,
    ) -> Self {
        WebSocketState {
            logs_clients: Arc::new(Mutex::new(HashMap::new())),
            status_clients: Arc::new(Mutex::new(HashMap::new())),
            log_buffer,
            process,
            server_state,
        }
    }

    pub fn add_logs_client(&self, id: String, tx: mpsc::UnboundedSender<String>) {
        let mut clients = self.logs_clients.lock().unwrap();
        clients.insert(id.clone(), tx);
        println!("✅ Logs WebSocket client connected: {} (total: {})", id, clients.len());
    }

    pub fn remove_logs_client(&self, id: &str) {
        let mut clients = self.logs_clients.lock().unwrap();
        clients.remove(id);
        println!("🔌 Logs WebSocket client disconnected: {} (remaining: {})", id, clients.len());
    }

    pub fn add_status_client(&self, id: String, tx: mpsc::UnboundedSender<String>) {
        let mut clients = self.status_clients.lock().unwrap();
        clients.insert(id, tx);
    }

    pub fn remove_status_client(&self, id: &str) {
        let mut clients = self.status_clients.lock().unwrap();
        clients.remove(id);
    }

    pub fn broadcast_log(&self, log: LogLine) {
        let clients = self.logs_clients.lock().unwrap();
        let client_count = clients.len();
        let message = serde_json::to_string(&WebSocketMessage::Log { log }).unwrap();
        let mut sent_count = 0;
        for tx in clients.values() {
            if tx.send(message.clone()).is_ok() {
                sent_count += 1;
            }
        }
        if client_count > 0 {
            println!("📤 Sent log to {}/{} WebSocket clients", sent_count, client_count);
        }
    }

    pub fn broadcast_status(&self, active: bool, port: u16) {
        let clients = self.status_clients.lock().unwrap();
        let message = serde_json::to_string(&WebSocketMessage::Status { active, port }).unwrap();
        for tx in clients.values() {
            let _ = tx.send(message.clone());
        }
    }
}

// Helper function to get status
fn get_status(
    process: &web::Data<ProcessHandle>,
    server_state: &web::Data<ServerStateHandle>,
) -> crate::api::llama_server::get_status::LlamaServerStatus {
    use crate::api::llama_server::get_status::LlamaServerStatus;

    let mut process_guard = process.lock().unwrap();

    let is_active = if let Some(ref mut child) = *process_guard {
        match child.try_wait() {
            Ok(Some(_)) => {
                drop(process_guard);
                let mut p = process.lock().unwrap();
                *p = None;
                false
            }
            Ok(None) => true,
            Err(_) => false,
        }
    } else {
        false
    };

    let state_guard = server_state.lock().unwrap();
    let is_ready = state_guard.is_ready;
    drop(state_guard);

    // Check port synchronously
    let port_check = std::net::TcpStream::connect("127.0.0.1:8080").is_ok();

    LlamaServerStatus {
        active: is_active && (is_ready || port_check),
        port: 8080,
    }
}

// Logs WebSocket handler
pub async fn logs_ws(
    state: web::Data<Arc<WebSocketState>>,
    session: Session,
    mut msg_stream: actix_ws::MessageStream,
) {
    use uuid::Uuid;
    
    let client_id = Uuid::new_v4().to_string();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Add client
    state.add_logs_client(client_id.clone(), tx);

    // Clone session for sending messages
    let mut session_sender = session.clone();

    // Send initial logs batch
    {
        let buffer = state.log_buffer.lock().unwrap();
        let logs: Vec<LogLine> = buffer
            .iter()
            .map(|entry| LogLine {
                timestamp: entry.timestamp,
                line: entry.line.clone(),
                source: match entry.source {
                    LogSource::Stdout => "stdout".to_string(),
                    LogSource::Stderr => "stderr".to_string(),
                },
            })
            .collect();
        drop(buffer);

        if !logs.is_empty() {
            let message = serde_json::to_string(&WebSocketMessage::LogsBatch { logs }).unwrap();
            let _ = session_sender.text(message).await;
        }
    }
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
                // Client messages are not needed for logs, just ignore
            }
            Message::Close(_) => {
                break;
            }
            _ => {}
        }
    }

    // Remove client on disconnect
    state.remove_logs_client(&client_id);
}

// Status WebSocket handler
pub async fn status_ws(
    state: web::Data<Arc<WebSocketState>>,
    session: Session,
    mut msg_stream: actix_ws::MessageStream,
) {
    use uuid::Uuid;
    
    let client_id = Uuid::new_v4().to_string();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Add client
    state.add_status_client(client_id.clone(), tx);

    // Clone session for sending messages
    let mut session_sender = session.clone();

    // Send initial status
    let status = get_status(&state.process, &state.server_state);
    let initial_message = serde_json::to_string(&WebSocketMessage::Status {
        active: status.active,
        port: status.port,
    })
    .unwrap();
    let _ = session_sender.text(initial_message).await;
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
            Message::Text(_) => {
                // Status is server-pushed, no need to handle client messages
            }
            Message::Close(_) => {
                break;
            }
            _ => {}
        }
    }

    // Remove client on disconnect
    state.remove_status_client(&client_id);
}

// HTTP handler for logs WebSocket
pub async fn logs_websocket(
    req: HttpRequest,
    stream: Payload,
    state: web::Data<Arc<WebSocketState>>,
) -> Result<HttpResponse, Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;
    let state_clone = state.clone();
    actix_rt::spawn(logs_ws(state_clone, session, msg_stream));
    Ok(res)
}

// HTTP handler for status WebSocket
pub async fn status_websocket(
    req: HttpRequest,
    stream: Payload,
    state: web::Data<Arc<WebSocketState>>,
) -> Result<HttpResponse, Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;
    let state_clone = state.clone();
    actix_rt::spawn(status_ws(state_clone, session, msg_stream));
    Ok(res)
}
