use actix::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use serde::Serialize;
use std::collections::HashSet;

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

// WebSocket server that manages all connections
#[derive(Default)]
pub struct WebSocketServer {
    sessions: HashSet<Addr<WebSocketSession>>,
}

impl Actor for WebSocketServer {
    type Context = Context<Self>;
}

impl Handler<Connect> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) {
        self.sessions.insert(msg.addr);
    }
}

impl Handler<Disconnect> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.sessions.remove(&msg.addr);
    }
}

impl Handler<BroadcastLog> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: BroadcastLog, _: &mut Context<Self>) {
        for addr in &self.sessions {
            addr.do_send(msg.clone());
        }
    }
}

impl Handler<BroadcastStatus> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: BroadcastStatus, _: &mut Context<Self>) {
        for addr in &self.sessions {
            addr.do_send(msg.clone());
        }
    }
}

// Messages
#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    pub addr: Addr<WebSocketSession>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub addr: Addr<WebSocketSession>,
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct BroadcastLog {
    pub log: LogLine,
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct BroadcastStatus {
    pub active: bool,
    pub port: u16,
}

// WebSocket session actor
pub struct WebSocketSession {
    pub server: Addr<WebSocketServer>,
    pub session_type: SessionType,
    pub log_buffer: Option<web::Data<LogBuffer>>,
    pub process: Option<web::Data<ProcessHandle>>,
    pub server_state: Option<web::Data<ServerStateHandle>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SessionType {
    Logs,
    Status,
}

impl Actor for WebSocketSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = ctx.address();
        self.server.do_send(Connect { addr });

        // Send initial data based on session type
        match self.session_type {
            SessionType::Logs => {
                if let Some(ref log_buffer) = self.log_buffer {
                    let buffer = log_buffer.lock().unwrap();
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
                        let msg = serde_json::to_string(&WebSocketMessage::LogsBatch { logs })
                            .unwrap();
                        ctx.text(msg);
                    }
                }
            }
            SessionType::Status => {
                if let (Some(ref process), Some(ref server_state)) =
                    (&self.process, &self.server_state)
                {
                    let status = get_status(process, server_state);
                    let msg = serde_json::to_string(&WebSocketMessage::Status {
                        active: status.active,
                        port: status.port,
                    })
                    .unwrap();
                    ctx.text(msg);
                }
            }
        }
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        self.server.do_send(Disconnect {
            addr: ctx.address(),
        });
        Running::Stop
    }
}

impl Handler<BroadcastLog> for WebSocketSession {
    type Result = ();

    fn handle(&mut self, msg: BroadcastLog, ctx: &mut Self::Context) {
        if self.session_type == SessionType::Logs {
            let ws_msg = WebSocketMessage::Log { log: msg.log };
            if let Ok(json) = serde_json::to_string(&ws_msg) {
                ctx.text(json);
            }
        }
    }
}

impl Handler<BroadcastStatus> for WebSocketSession {
    type Result = ();

    fn handle(&mut self, msg: BroadcastStatus, ctx: &mut Self::Context) {
        if self.session_type == SessionType::Status {
            let ws_msg = WebSocketMessage::Status {
                active: msg.active,
                port: msg.port,
            };
            if let Ok(json) = serde_json::to_string(&ws_msg) {
                ctx.text(json);
            }
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Pong(_)) => {}
            Ok(ws::Message::Text(_)) => {}
            Ok(ws::Message::Binary(_)) => {}
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

// Helper function to get status
fn get_status(process: &web::Data<ProcessHandle>, server_state: &web::Data<ServerStateHandle>) -> crate::api::llama_server::get_status::LlamaServerStatus {
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
    
    // Check port synchronously (we'll use a simple check)
    let port_check = std::net::TcpStream::connect("127.0.0.1:8080").is_ok();

    LlamaServerStatus {
        active: is_active && (is_ready || port_check),
        port: 8080,
    }
}

// HTTP handler for logs WebSocket
pub async fn logs_websocket(
    req: HttpRequest,
    stream: web::Payload,
    server: web::Data<Addr<WebSocketServer>>,
    log_buffer: web::Data<LogBuffer>,
) -> Result<HttpResponse, Error> {
    let resp = ws::start(
        WebSocketSession {
            server: server.get_ref().clone(),
            session_type: SessionType::Logs,
            log_buffer: Some(log_buffer),
            process: None,
            server_state: None,
        },
        &req,
        stream,
    )?;
    Ok(resp)
}

// HTTP handler for status WebSocket
pub async fn status_websocket(
    req: HttpRequest,
    stream: web::Payload,
    server: web::Data<Addr<WebSocketServer>>,
    process: web::Data<ProcessHandle>,
    server_state: web::Data<ServerStateHandle>,
) -> Result<HttpResponse, Error> {
    let resp = ws::start(
        WebSocketSession {
            server: server.get_ref().clone(),
            session_type: SessionType::Status,
            log_buffer: None,
            process: Some(process),
            server_state: Some(server_state),
        },
        &req,
        stream,
    )?;
    Ok(resp)
}

