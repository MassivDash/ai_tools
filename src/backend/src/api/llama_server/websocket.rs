use actix_web::{web, web::Payload, Error, HttpRequest, HttpResponse};
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
        let count = {
            let mut clients = self.logs_clients.lock().unwrap_or_else(|e| e.into_inner());
            clients.insert(id.clone(), tx);
            clients.len()
        };
        println!(
            "✅ Logs WebSocket client connected: {} (total: {})",
            id, count
        );
    }

    pub fn remove_logs_client(&self, id: &str) {
        let (removed, count) = {
            let mut clients = self.logs_clients.lock().unwrap_or_else(|e| e.into_inner());
            let removed = clients.remove(id).is_some();
            (removed, clients.len())
        };
        if removed {
            println!(
                "🔌 Logs WebSocket client disconnected: {} (remaining: {})",
                id, count
            );
        }
    }

    pub fn add_status_client(&self, id: String, tx: mpsc::UnboundedSender<String>) {
        let mut clients = self
            .status_clients
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        clients.insert(id, tx);
    }

    pub fn remove_status_client(&self, id: &str) {
        let mut clients = self
            .status_clients
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        clients.remove(id);
    }

    pub fn broadcast_log(&self, log: LogLine) {
        let (sent_count, client_count) = {
            let clients = self.logs_clients.lock().unwrap_or_else(|e| e.into_inner());
            let client_count = clients.len();
            let message = serde_json::to_string(&WebSocketMessage::Log { log }).unwrap();
            let mut sent_count = 0;
            for tx in clients.values() {
                if tx.send(message.clone()).is_ok() {
                    sent_count += 1;
                }
            }
            (sent_count, client_count)
        };
        if client_count > 0 {
            println!(
                "📤 Sent log to {}/{} WebSocket clients",
                sent_count, client_count
            );
        }
    }

    pub fn broadcast_status(&self, active: bool, port: u16) {
        let clients = self
            .status_clients
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let message = serde_json::to_string(&WebSocketMessage::Status { active, port }).unwrap();
        for tx in clients.values() {
            let _ = tx.send(message.clone());
        }
    }
}

/// Snapshots the buffered log entries in the shape the WebSocket protocol uses.
fn snapshot_logs(log_buffer: &LogBuffer) -> Vec<LogLine> {
    let buffer = log_buffer.lock().unwrap();
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
    let logs = snapshot_logs(&state.log_buffer);

    if !logs.is_empty() {
        let message = serde_json::to_string(&WebSocketMessage::LogsBatch { logs }).unwrap();
        let _ = session_sender.text(message).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::llama_server::types::{LogEntry, ServerState};
    use std::collections::VecDeque;
    use std::process::{Command, Stdio};
    use tokio::sync::mpsc::UnboundedReceiver;

    /// A trivially cheap, short-lived process, used only so the status helper has
    /// a real live `Child` to inspect. Never `llama-server`.
    fn spawn_harmless_child(seconds: &str) -> std::process::Child {
        Command::new("sleep")
            .arg(seconds)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("`sleep` must be available to run these tests")
    }

    struct Fixture {
        state: WebSocketState,
        log_buffer: LogBuffer,
        process: web::Data<ProcessHandle>,
        server_state: ServerStateHandle,
    }

    fn fixture() -> Fixture {
        let log_buffer: LogBuffer = Arc::new(Mutex::new(VecDeque::new()));
        let process = ProcessHandle(Arc::new(Mutex::new(None)));
        let server_state: ServerStateHandle = Arc::new(Mutex::new(ServerState {
            is_ready: false,
            generation: 0,
        }));
        let process_data = web::Data::new(process.clone());

        Fixture {
            state: WebSocketState::new(
                web::Data::new(log_buffer.clone()),
                process_data.clone(),
                web::Data::new(server_state.clone()),
            ),
            log_buffer,
            process: process_data,
            server_state,
        }
    }

    fn drain(rx: &mut UnboundedReceiver<String>) -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            out.push(serde_json::from_str(&msg).unwrap());
        }
        out
    }

    fn log_line(line: &str) -> LogLine {
        LogLine {
            timestamp: 1700000000,
            line: line.to_string(),
            source: "stdout".to_string(),
        }
    }

    #[test]
    fn test_new_starts_with_no_registered_clients() {
        let f = fixture();

        assert!(f.state.logs_clients.lock().unwrap().is_empty());
        assert!(f.state.status_clients.lock().unwrap().is_empty());
    }

    #[test]
    fn test_logs_clients_are_registered_and_removed_by_id() {
        let f = fixture();
        let (tx_a, _rx_a) = mpsc::unbounded_channel();
        let (tx_b, _rx_b) = mpsc::unbounded_channel();

        f.state.add_logs_client("a".to_string(), tx_a);
        f.state.add_logs_client("b".to_string(), tx_b);
        assert_eq!(f.state.logs_clients.lock().unwrap().len(), 2);

        f.state.remove_logs_client("a");
        let clients = f.state.logs_clients.lock().unwrap();
        assert_eq!(clients.len(), 1);
        assert!(clients.contains_key("b"));
    }

    #[test]
    fn test_removing_an_unknown_logs_client_is_a_no_op() {
        let f = fixture();
        let (tx, _rx) = mpsc::unbounded_channel();
        f.state.add_logs_client("a".to_string(), tx);

        f.state.remove_logs_client("does-not-exist");

        assert_eq!(f.state.logs_clients.lock().unwrap().len(), 1);
    }

    #[test]
    fn test_re_adding_the_same_logs_client_id_replaces_the_sender() {
        let f = fixture();
        let (tx_old, mut rx_old) = mpsc::unbounded_channel();
        let (tx_new, mut rx_new) = mpsc::unbounded_channel();
        f.state.add_logs_client("same".to_string(), tx_old);
        f.state.add_logs_client("same".to_string(), tx_new);

        f.state.broadcast_log(log_line("hello"));

        assert_eq!(f.state.logs_clients.lock().unwrap().len(), 1);
        assert!(drain(&mut rx_old).is_empty());
        assert_eq!(drain(&mut rx_new).len(), 1);
    }

    #[test]
    fn test_status_clients_are_registered_and_removed_by_id() {
        let f = fixture();
        let (tx, _rx) = mpsc::unbounded_channel();

        f.state.add_status_client("s1".to_string(), tx);
        assert_eq!(f.state.status_clients.lock().unwrap().len(), 1);

        f.state.remove_status_client("s1");
        assert!(f.state.status_clients.lock().unwrap().is_empty());
    }

    #[test]
    fn test_broadcast_log_reaches_every_logs_client() {
        let f = fixture();
        let (tx_a, mut rx_a) = mpsc::unbounded_channel();
        let (tx_b, mut rx_b) = mpsc::unbounded_channel();
        f.state.add_logs_client("a".to_string(), tx_a);
        f.state.add_logs_client("b".to_string(), tx_b);

        f.state.broadcast_log(LogLine {
            timestamp: 42,
            line: "loading model".to_string(),
            source: "stderr".to_string(),
        });

        for rx in [&mut rx_a, &mut rx_b] {
            let msgs = drain(rx);
            assert_eq!(msgs.len(), 1);
            assert_eq!(msgs[0]["type"], "log");
            assert_eq!(msgs[0]["log"]["timestamp"], 42);
            assert_eq!(msgs[0]["log"]["line"], "loading model");
            assert_eq!(msgs[0]["log"]["source"], "stderr");
        }
    }

    #[test]
    fn test_broadcast_log_does_not_reach_status_clients() {
        let f = fixture();
        let (tx, mut rx) = mpsc::unbounded_channel();
        f.state.add_status_client("s1".to_string(), tx);

        f.state.broadcast_log(log_line("hello"));

        assert!(drain(&mut rx).is_empty());
    }

    #[test]
    fn test_broadcast_log_tolerates_a_dropped_receiver() {
        let f = fixture();
        let (tx_dead, rx_dead) = mpsc::unbounded_channel();
        let (tx_live, mut rx_live) = mpsc::unbounded_channel();
        f.state.add_logs_client("dead".to_string(), tx_dead);
        f.state.add_logs_client("live".to_string(), tx_live);
        drop(rx_dead);

        f.state.broadcast_log(log_line("still delivered"));

        assert_eq!(drain(&mut rx_live).len(), 1);
        // The dead client is not evicted, only skipped.
        assert_eq!(f.state.logs_clients.lock().unwrap().len(), 2);
    }

    #[test]
    fn test_broadcast_status_reaches_every_status_client() {
        let f = fixture();
        let (tx, mut rx) = mpsc::unbounded_channel();
        f.state.add_status_client("s1".to_string(), tx);

        f.state.broadcast_status(true, 8099);

        let msgs = drain(&mut rx);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["type"], "status");
        assert_eq!(msgs[0]["active"], true);
        assert_eq!(msgs[0]["port"], 8099);
    }

    #[test]
    fn test_broadcast_with_no_clients_is_harmless() {
        let f = fixture();

        f.state.broadcast_log(log_line("nobody listening"));
        f.state.broadcast_status(false, 8080);

        assert!(f.state.logs_clients.lock().unwrap().is_empty());
    }

    #[test]
    fn test_logs_batch_message_serialises_as_a_tagged_envelope() {
        let json = serde_json::to_value(WebSocketMessage::LogsBatch {
            logs: vec![log_line("first"), log_line("second")],
        })
        .unwrap();

        assert_eq!(json["type"], "logs_batch");
        assert_eq!(json["logs"].as_array().unwrap().len(), 2);
        assert_eq!(json["logs"][1]["line"], "second");
    }

    #[test]
    fn test_snapshot_logs_maps_the_buffer_and_labels_both_sources() {
        let f = fixture();
        {
            let mut buffer = f.log_buffer.lock().unwrap();
            buffer.push_back(LogEntry {
                timestamp: 1,
                line: "out".to_string(),
                source: LogSource::Stdout,
            });
            buffer.push_back(LogEntry {
                timestamp: 2,
                line: "err".to_string(),
                source: LogSource::Stderr,
            });
        }

        let snapshot = snapshot_logs(&f.log_buffer);

        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].source, "stdout");
        assert_eq!(snapshot[0].line, "out");
        assert_eq!(snapshot[1].source, "stderr");
        assert_eq!(snapshot[1].timestamp, 2);
    }

    #[test]
    fn test_snapshot_logs_of_an_empty_buffer_is_empty() {
        let f = fixture();

        assert!(snapshot_logs(&f.log_buffer).is_empty());
    }

    #[test]
    fn test_get_status_is_inactive_without_a_managed_process() {
        let f = fixture();

        let status = get_status(&f.process, &web::Data::new(f.server_state.clone()));

        assert!(!status.active);
        assert_eq!(status.port, 8080);
    }

    #[test]
    fn test_get_status_is_active_for_a_live_process_that_reported_ready() {
        let f = fixture();
        *f.process.lock().unwrap() = Some(spawn_harmless_child("5"));
        f.server_state.lock().unwrap().is_ready = true;

        let status = get_status(&f.process, &web::Data::new(f.server_state.clone()));

        assert!(status.active);

        if let Some(child) = f.process.lock().unwrap().as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        };
    }

    #[test]
    fn test_get_status_clears_the_handle_once_the_process_has_exited() {
        let f = fixture();
        let mut child = spawn_harmless_child("0");
        let _ = child.wait();
        *f.process.lock().unwrap() = Some(child);
        f.server_state.lock().unwrap().is_ready = true;

        let status = get_status(&f.process, &web::Data::new(f.server_state.clone()));

        assert!(!status.active);
        assert!(
            f.process.lock().unwrap().is_none(),
            "an exited process should be cleared from the handle"
        );
    }
}
