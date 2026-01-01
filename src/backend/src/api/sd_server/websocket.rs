use actix_web::{web, web::Payload, Error, HttpRequest, HttpResponse};
use actix_ws::{Message, Session};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use crate::api::sd_server::types::{
    LogBuffer, LogSource, SDConfigHandle, SDProcessHandle, SDStateHandle,
};

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
    #[serde(rename = "logs_batch")]
    LogsBatch { logs: Vec<LogLine> },
    #[serde(rename = "status")]
    Status {
        is_generating: bool,
        current_file: Option<String>,
    },
}

#[derive(Clone)]
pub struct WebSocketState {
    pub clients: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>,
    pub log_buffer: web::Data<LogBuffer>,
    #[allow(dead_code)]
    pub process: web::Data<SDProcessHandle>,
    #[allow(dead_code)]
    pub config: web::Data<SDConfigHandle>,
    pub state: web::Data<SDStateHandle>,
}

impl WebSocketState {
    pub fn new(
        log_buffer: web::Data<LogBuffer>,
        process: web::Data<SDProcessHandle>,
        config: web::Data<SDConfigHandle>,
        state: web::Data<SDStateHandle>,
    ) -> Self {
        WebSocketState {
            clients: Arc::new(Mutex::new(HashMap::new())),
            log_buffer,
            process,
            config,
            state,
        }
    }

    pub fn add_client(&self, id: String, tx: mpsc::UnboundedSender<String>) {
        let mut clients = self.clients.lock().unwrap();
        clients.insert(id.clone(), tx);
    }

    pub fn remove_client(&self, id: &str) {
        let mut clients = self.clients.lock().unwrap();
        clients.remove(id);
    }

    pub fn broadcast_log(&self, log: LogLine) {
        let clients = self.clients.lock().unwrap();
        let message = serde_json::to_string(&WebSocketMessage::Log { log }).unwrap();
        for tx in clients.values() {
            let _ = tx.send(message.clone());
        }
    }

    pub fn broadcast_status(&self, is_generating: bool, current_file: Option<String>) {
        let clients = self.clients.lock().unwrap();
        let message = serde_json::to_string(&WebSocketMessage::Status {
            is_generating,
            current_file,
        })
        .unwrap();
        for tx in clients.values() {
            let _ = tx.send(message.clone());
        }
    }
}

// Logs WebSocket handler
pub async fn ws_handler(
    state: web::Data<Arc<WebSocketState>>,
    session: Session,
    mut msg_stream: actix_ws::MessageStream,
) {
    use uuid::Uuid;

    let client_id = Uuid::new_v4().to_string();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Add client
    state.add_client(client_id.clone(), tx);

    // Clone session for sending messages
    let mut session_sender = session.clone();

    // Send initial logs batch
    let logs: Vec<LogLine> = {
        let buffer = state.log_buffer.lock().unwrap();
        buffer
            .iter()
            .map(|entry| LogLine {
                timestamp: entry.timestamp,
                line: entry.line.clone(),
                source: match entry.source {
                    LogSource::Stdout => "stdout".to_string(),
                    LogSource::Stderr => "stderr".to_string(),
                },
            })
            .collect()
    };

    if !logs.is_empty() {
        let message = serde_json::to_string(&WebSocketMessage::LogsBatch { logs }).unwrap();
        let _ = session_sender.text(message).await;
    }

    // Send initial status
    let status_msg = {
        let state_guard = state.state.lock().unwrap();
        serde_json::to_string(&WebSocketMessage::Status {
            is_generating: state_guard.is_generating,
            current_file: state_guard.current_output_file.clone(),
        })
        .unwrap()
    };
    let _ = session_sender.text(status_msg).await;

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
    state.remove_client(&client_id);
}

// HTTP handler for logs WebSocket
pub async fn sd_logs_ws(
    req: HttpRequest,
    stream: Payload,
    state: web::Data<Arc<WebSocketState>>,
) -> Result<HttpResponse, Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;
    let state_clone = state.clone();
    actix_rt::spawn(ws_handler(state_clone, session, msg_stream));
    Ok(res)
}
