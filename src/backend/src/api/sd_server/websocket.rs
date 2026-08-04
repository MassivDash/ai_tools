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
    #[serde(rename = "error")]
    Error { message: String },
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

    pub fn broadcast_error(&self, message: String) {
        let clients = self.clients.lock().unwrap();
        let payload = serde_json::to_string(&WebSocketMessage::Error { message }).unwrap();
        for tx in clients.values() {
            let _ = tx.send(payload.clone());
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
    let logs = snapshot_logs(&state.log_buffer);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::sd_server::types::{LogEntry, SDConfig, SDState};
    use std::collections::VecDeque;
    use tokio::sync::mpsc::UnboundedReceiver;

    struct Fixture {
        state: WebSocketState,
        log_buffer: LogBuffer,
        sd_state: SDStateHandle,
    }

    fn fixture() -> Fixture {
        let log_buffer: LogBuffer = Arc::new(Mutex::new(VecDeque::new()));
        let sd_state: SDStateHandle = Arc::new(Mutex::new(SDState {
            is_generating: false,
            current_output_file: None,
            pending_filename: None,
        }));

        Fixture {
            state: WebSocketState::new(
                web::Data::new(log_buffer.clone()),
                web::Data::new(Arc::new(Mutex::new(None))),
                web::Data::new(Arc::new(Mutex::new(SDConfig::default()))),
                web::Data::new(sd_state.clone()),
            ),
            log_buffer,
            sd_state,
        }
    }

    fn drain(rx: &mut UnboundedReceiver<String>) -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            out.push(serde_json::from_str(&msg).unwrap());
        }
        out
    }

    #[test]
    fn test_new_starts_with_no_clients_and_shares_the_given_state() {
        let f = fixture();

        assert!(f.state.clients.lock().unwrap().is_empty());
        f.sd_state.lock().unwrap().is_generating = true;
        assert!(f.state.state.lock().unwrap().is_generating);
    }

    #[test]
    fn test_clients_are_registered_and_removed_by_id() {
        let f = fixture();
        let (tx_a, _rx_a) = mpsc::unbounded_channel();
        let (tx_b, _rx_b) = mpsc::unbounded_channel();

        f.state.add_client("a".to_string(), tx_a);
        f.state.add_client("b".to_string(), tx_b);
        assert_eq!(f.state.clients.lock().unwrap().len(), 2);

        f.state.remove_client("a");
        let clients = f.state.clients.lock().unwrap();
        assert_eq!(clients.len(), 1);
        assert!(clients.contains_key("b"));
    }

    #[test]
    fn test_removing_an_unknown_client_is_a_no_op() {
        let f = fixture();
        let (tx, _rx) = mpsc::unbounded_channel();
        f.state.add_client("a".to_string(), tx);

        f.state.remove_client("nope");

        assert_eq!(f.state.clients.lock().unwrap().len(), 1);
    }

    #[test]
    fn test_broadcast_log_reaches_every_client() {
        let f = fixture();
        let (tx_a, mut rx_a) = mpsc::unbounded_channel();
        let (tx_b, mut rx_b) = mpsc::unbounded_channel();
        f.state.add_client("a".to_string(), tx_a);
        f.state.add_client("b".to_string(), tx_b);

        f.state.broadcast_log(LogLine {
            timestamp: 7,
            line: "step 1/20".to_string(),
            source: "stderr".to_string(),
        });

        for rx in [&mut rx_a, &mut rx_b] {
            let msgs = drain(rx);
            assert_eq!(msgs.len(), 1);
            assert_eq!(msgs[0]["type"], "log");
            assert_eq!(msgs[0]["log"]["timestamp"], 7);
            assert_eq!(msgs[0]["log"]["line"], "step 1/20");
            assert_eq!(msgs[0]["log"]["source"], "stderr");
        }
    }

    #[test]
    fn test_broadcast_status_carries_the_current_file() {
        let f = fixture();
        let (tx, mut rx) = mpsc::unbounded_channel();
        f.state.add_client("a".to_string(), tx);

        f.state
            .broadcast_status(false, Some("output_1.png".to_string()));

        let msgs = drain(&mut rx);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["type"], "status");
        assert_eq!(msgs[0]["is_generating"], false);
        assert_eq!(msgs[0]["current_file"], "output_1.png");
    }

    #[test]
    fn test_broadcast_error_is_tagged_as_an_error() {
        let f = fixture();
        let (tx, mut rx) = mpsc::unbounded_channel();
        f.state.add_client("a".to_string(), tx);

        f.state.broadcast_error("out of memory".to_string());

        let msgs = drain(&mut rx);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["type"], "error");
        assert_eq!(msgs[0]["message"], "out of memory");
    }

    #[test]
    fn test_broadcasts_tolerate_a_dropped_receiver() {
        let f = fixture();
        let (tx_dead, rx_dead) = mpsc::unbounded_channel();
        let (tx_live, mut rx_live) = mpsc::unbounded_channel();
        f.state.add_client("dead".to_string(), tx_dead);
        f.state.add_client("live".to_string(), tx_live);
        drop(rx_dead);

        f.state.broadcast_status(true, None);
        f.state.broadcast_error("boom".to_string());

        assert_eq!(drain(&mut rx_live).len(), 2);
        // A dead client is skipped, not evicted.
        assert_eq!(f.state.clients.lock().unwrap().len(), 2);
    }

    #[test]
    fn test_broadcast_with_no_clients_is_harmless() {
        let f = fixture();

        f.state.broadcast_log(LogLine {
            timestamp: 1,
            line: "nobody listening".to_string(),
            source: "stdout".to_string(),
        });
        f.state.broadcast_status(true, None);
        f.state.broadcast_error("boom".to_string());

        assert!(f.state.clients.lock().unwrap().is_empty());
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
    fn test_logs_batch_message_serialises_as_a_tagged_envelope() {
        let json = serde_json::to_value(WebSocketMessage::LogsBatch {
            logs: vec![LogLine {
                timestamp: 3,
                line: "only".to_string(),
                source: "stdout".to_string(),
            }],
        })
        .unwrap();

        assert_eq!(json["type"], "logs_batch");
        assert_eq!(json["logs"].as_array().unwrap().len(), 1);
        assert_eq!(json["logs"][0]["line"], "only");
    }
}
