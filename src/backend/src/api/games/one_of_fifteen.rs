use actix::prelude::*;
use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

// --- Game State Structures ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Presenter,
    Contestant,
    Viewer, // Default
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contestant {
    pub name: String,
    pub score: i32,
    pub id: String, // WebSocket Session ID (or random UUID) - TO BE REMOVED/MIGRATED to session logic? No, keeping as connection ID for now, but need session_id for persistence.
    // Actually, let's use `id` as the persistent session_id.
    pub session_id: String,
    pub online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GameStatus {
    Lobby,
    Playing,
    Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub presenter_id: Option<String>, // Session ID of the presenter
    pub presenter_online: bool,
    pub contestants: HashMap<String, Contestant>, // Keyed by session_id
    pub active: bool,
    pub status: GameStatus,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            presenter_id: None,
            presenter_online: false,
            contestants: HashMap::new(),
            active: true,
            status: GameStatus::Lobby,
        }
    }
}

pub type GameStateHandle = Arc<Mutex<GameState>>;

// --- WebSocket Messages (Incoming from Client) ---

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IncomingMessage {
    Identify { session_id: String },
    JoinPresenter,
    JoinContestant { name: String },
    StartGame,
    ResetGame,
    GetState,
}

// --- WebSocket Messages (Outgoing to Client) ---

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutgoingMessage {
    StateUpdate(GameStateSnapshot),
    Error { message: String },
    Welcome { role: UserRole },
}

#[derive(Debug, Serialize, Clone)]
pub struct GameStateSnapshot {
    pub has_presenter: bool,
    pub presenter_online: bool, // Added
    pub contestants: Vec<Contestant>,
    pub status: GameStatus,
}

// --- Actor ---

pub struct OneOfFifteenWebSocket {
    hb: Instant,
    state: GameStateHandle,
    id: String,
    role: UserRole,
}

impl OneOfFifteenWebSocket {
    pub fn new(state: GameStateHandle) -> Self {
        Self {
            hb: Instant::now(),
            state,
            id: uuid::Uuid::new_v4().to_string(), // Unique ID for this connection
            role: UserRole::Viewer,
        }
    }

    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }

    fn send_error(&self, ctx: &mut ws::WebsocketContext<Self>, message: &str) {
        let msg = OutgoingMessage::Error {
            message: message.to_string(),
        };
        if let Ok(json) = serde_json::to_string(&msg) {
            ctx.text(json);
        }
    }
}

impl Actor for OneOfFifteenWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
        // On connect, just send current state readiness?
    }

    fn stopping(&mut self, _: &mut Self::Context) -> actix::Running {
        // Cleanup role -> Mark as offline instead of removing
        let mut state = self.state.lock().unwrap();
        // We use self.id as the session_id for now if we didn't implement separate session tracking yet?
        // Wait, self.id was just UUID.
        // If we want persistence, self.id should probably BE the session_id provided by client.
        // But `stopping` runs when actor dies.
        // If I update `self.id` to be the session_id?

        match self.role {
            UserRole::Presenter => {
                if state.presenter_id.as_ref() == Some(&self.id) {
                    state.presenter_online = false;
                }
            }
            UserRole::Contestant => {
                if let Some(c) = state.contestants.get_mut(&self.id) {
                    c.online = false;
                }
            }
            _ => {}
        }
        actix::Running::Stop
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for OneOfFifteenWebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                if let Ok(input) = serde_json::from_str::<IncomingMessage>(&text) {
                    let mut state = self.state.lock().unwrap();

                    match input {
                        IncomingMessage::Identify { session_id } => {
                            // Adopt the session ID
                            self.id = session_id.clone();

                            // Restore Role
                            if state.presenter_id.as_ref() == Some(&self.id) {
                                self.role = UserRole::Presenter;
                                state.presenter_online = true;
                                let msg = OutgoingMessage::Welcome {
                                    role: UserRole::Presenter,
                                };
                                ctx.text(serde_json::to_string(&msg).unwrap());
                            } else if let Some(contestant) = state.contestants.get_mut(&self.id) {
                                self.role = UserRole::Contestant;
                                contestant.online = true;
                                let msg = OutgoingMessage::Welcome {
                                    role: UserRole::Contestant,
                                };
                                ctx.text(serde_json::to_string(&msg).unwrap());
                            } else {
                                // Session not found or is just a viewer.
                                self.send_error(ctx, "Session not found");
                            }
                        }
                        IncomingMessage::JoinPresenter => {
                            println!("🎤 JoinPresenter request from {}", self.id);

                            // Check if I am currently a contestant? If so, remove me.
                            if state.contestants.contains_key(&self.id) {
                                println!("⚠️ Switching from Contestant to Presenter: {}", self.id);
                                state.contestants.remove(&self.id);
                            }

                            if let Some(pid) = &state.presenter_id {
                                if pid == &self.id {
                                    // Already presenter, just update online status
                                    state.presenter_online = true;
                                    self.role = UserRole::Presenter;
                                    let msg = OutgoingMessage::Welcome {
                                        role: UserRole::Presenter,
                                    };
                                    ctx.text(serde_json::to_string(&msg).unwrap());
                                    return;
                                }
                                if state.presenter_online {
                                    self.send_error(ctx, "Presenter already exists and is online");
                                    return;
                                }
                                // If offline, we can potentially steal it if we want strict single-presenter logic?
                                // usage: self.send_error(ctx, "Presenter role is reserved");
                                // For now, stick to reserved.
                                self.send_error(ctx, "Presenter role is reserved");
                            } else {
                                println!("👑 New Presenter assigned: {}", self.id);
                                state.presenter_id = Some(self.id.clone());
                                state.presenter_online = true;
                                self.role = UserRole::Presenter;
                                let msg = OutgoingMessage::Welcome {
                                    role: UserRole::Presenter,
                                };
                                ctx.text(serde_json::to_string(&msg).unwrap());
                            }
                        }
                        IncomingMessage::JoinContestant { name } => {
                            println!("👤 JoinContestant request: {} ({})", name, self.id);

                            // Check if I am currently Presenter? If so, resign.
                            if state.presenter_id.as_ref() == Some(&self.id) {
                                println!(
                                    "⚠️ Resigning from Presenter to join as Contestant: {}",
                                    self.id
                                );
                                state.presenter_id = None;
                                state.presenter_online = false;
                            }

                            // If I am already a contestant (re-join via button?), update name
                            let session_id = self.id.clone();

                            let contestant = Contestant {
                                name,
                                score: 0,
                                id: session_id.clone(),
                                session_id: session_id.clone(),
                                online: true,
                            };
                            state.contestants.insert(session_id.clone(), contestant);
                            self.role = UserRole::Contestant;
                            let msg = OutgoingMessage::Welcome {
                                role: UserRole::Contestant,
                            };
                            ctx.text(serde_json::to_string(&msg).unwrap());
                        }
                        IncomingMessage::StartGame => {
                            if state.presenter_id.as_ref() == Some(&self.id) {
                                state.status = GameStatus::Playing;
                                // Broadcast state logic would go here if we were pushing, but clients poll.
                            }
                        }
                        IncomingMessage::ResetGame => {
                            if state.presenter_id.as_ref() == Some(&self.id) {
                                state.status = GameStatus::Lobby;
                                state.contestants.values_mut().for_each(|c| c.score = 0);
                            }
                        }
                        IncomingMessage::GetState => {
                            let snapshot = GameStateSnapshot {
                                has_presenter: state.presenter_id.is_some(),
                                presenter_online: state.presenter_online,
                                contestants: state.contestants.values().cloned().collect(),
                                status: state.status.clone(),
                            };
                            let msg = OutgoingMessage::StateUpdate(snapshot);
                            if let Ok(json) = serde_json::to_string(&msg) {
                                ctx.text(json);
                            }
                        }
                    }
                }
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

#[get("/api/games/1-of-15/ws")]
pub async fn one_of_fifteen_ws_route(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<GameStateHandle>,
) -> Result<HttpResponse, Error> {
    ws::start(
        OneOfFifteenWebSocket::new(state.get_ref().clone()),
        &req,
        stream,
    )
}
