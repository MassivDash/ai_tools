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
#[serde(rename_all = "snake_case")]
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
    pub ready: bool,
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
    ToggleReady,
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

    fn process_message(
        msg: IncomingMessage,
        state: &mut GameState,
        connection_id: &mut String,
        connection_role: &mut UserRole,
    ) -> Vec<OutgoingMessage> {
        let mut responses = Vec::new();

        match msg {
            IncomingMessage::Identify { session_id } => {
                // Adopt the session ID
                *connection_id = session_id.clone();

                // Restore Role
                if state.presenter_id.as_ref() == Some(connection_id) {
                    *connection_role = UserRole::Presenter;
                    state.presenter_online = true;
                    responses.push(OutgoingMessage::Welcome {
                        role: UserRole::Presenter,
                    });
                } else if let Some(contestant) = state.contestants.get_mut(connection_id) {
                    *connection_role = UserRole::Contestant;
                    contestant.online = true;
                    responses.push(OutgoingMessage::Welcome {
                        role: UserRole::Contestant,
                    });
                } else {
                    // Session not found or is just a viewer.
                    responses.push(OutgoingMessage::Error {
                        message: "Session not found".to_string(),
                    });
                }
            }
            IncomingMessage::JoinPresenter => {
                // Check if I am currently a contestant? If so, remove me.
                if state.contestants.contains_key(connection_id) {
                    state.contestants.remove(connection_id);
                }

                if let Some(pid) = &state.presenter_id {
                    if pid == connection_id {
                        // Already presenter, just update online status
                        state.presenter_online = true;
                        *connection_role = UserRole::Presenter;
                        responses.push(OutgoingMessage::Welcome {
                            role: UserRole::Presenter,
                        });
                        return responses;
                    }
                    if state.presenter_online {
                        responses.push(OutgoingMessage::Error {
                            message: "Presenter already exists and is online".to_string(),
                        });
                        return responses;
                    }
                    // If offline, we can potentially steal it if we want strict single-presenter logic?
                    responses.push(OutgoingMessage::Error {
                        message: "Presenter role is reserved".to_string(),
                    });
                } else {
                    state.presenter_id = Some(connection_id.clone());
                    state.presenter_online = true;
                    *connection_role = UserRole::Presenter;
                    responses.push(OutgoingMessage::Welcome {
                        role: UserRole::Presenter,
                    });
                }
            }
            IncomingMessage::JoinContestant { name } => {
                // Check if I am currently Presenter? If so, resign.
                if state.presenter_id.as_ref() == Some(connection_id) {
                    state.presenter_id = None;
                    state.presenter_online = false;
                }

                // If I am already a contestant (re-join via button?), update name
                let session_id = connection_id.clone();

                let contestant = Contestant {
                    name,
                    score: 0,
                    id: session_id.clone(),
                    session_id: session_id.clone(),
                    online: true,
                    ready: false,
                };
                state.contestants.insert(session_id.clone(), contestant);
                *connection_role = UserRole::Contestant;
                responses.push(OutgoingMessage::Welcome {
                    role: UserRole::Contestant,
                });
            }
            IncomingMessage::StartGame => {
                if state.presenter_id.as_ref() == Some(connection_id) {
                    state.status = GameStatus::Playing;
                    // Broadcast state logic would go here if we were pushing, but clients poll.
                }
            }
            IncomingMessage::ResetGame => {
                if state.presenter_id.as_ref() == Some(connection_id) {
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
                responses.push(OutgoingMessage::StateUpdate(snapshot));
            }
            IncomingMessage::ToggleReady => {
                if let Some(c) = state.contestants.get_mut(connection_id) {
                    c.ready = !c.ready;
                    let snapshot = GameStateSnapshot {
                        has_presenter: state.presenter_id.is_some(),
                        presenter_online: state.presenter_online,
                        contestants: state.contestants.values().cloned().collect(),
                        status: state.status.clone(),
                    };
                    responses.push(OutgoingMessage::StateUpdate(snapshot));
                }
            }
        }
        responses
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
                    let responses =
                        Self::process_message(input, &mut state, &mut self.id, &mut self.role);

                    for msg in responses {
                        if let Ok(json) = serde_json::to_string(&msg) {
                            ctx.text(json);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_presenter_and_resume() {
        let mut state = GameState::new();
        let mut id = "session_1".to_string();
        let mut role = UserRole::Viewer;

        // 1. Join Presenter
        let msgs = OneOfFifteenWebSocket::process_message(
            IncomingMessage::JoinPresenter,
            &mut state,
            &mut id,
            &mut role,
        );

        // Verify state
        assert_eq!(state.presenter_id, Some("session_1".to_string()));
        assert!(state.presenter_online);
        assert_eq!(role, UserRole::Presenter);
        assert!(matches!(
            msgs[0],
            OutgoingMessage::Welcome {
                role: UserRole::Presenter
            }
        ));

        // 2. Simulate Disconnect (mark offline)
        state.presenter_online = false;

        // 3. New connection, Identify with same ID
        let mut new_id = "session_1".to_string(); // Same Session ID
        let mut new_role = UserRole::Viewer; // Default logic role
        let msgs = OneOfFifteenWebSocket::process_message(
            IncomingMessage::Identify {
                session_id: "session_1".to_string(),
            },
            &mut state,
            &mut new_id,
            &mut new_role,
        );

        // Verify Resumption
        assert_eq!(new_role, UserRole::Presenter);
        assert!(state.presenter_online); // Should be marked online again
        assert!(matches!(
            msgs[0],
            OutgoingMessage::Welcome {
                role: UserRole::Presenter
            }
        ));
    }

    #[test]
    fn test_join_contestant_and_resume() {
        let mut state = GameState::new();
        let mut id = "session_2".to_string();
        let mut role = UserRole::Viewer;

        // 1. Join Contestant
        let msgs = OneOfFifteenWebSocket::process_message(
            IncomingMessage::JoinContestant {
                name: "Alice".to_string(),
            },
            &mut state,
            &mut id,
            &mut role,
        );

        // Verify state
        assert!(state.contestants.contains_key("session_2"));
        assert_eq!(state.contestants.get("session_2").unwrap().name, "Alice");
        assert_eq!(role, UserRole::Contestant);
        assert!(matches!(
            msgs[0],
            OutgoingMessage::Welcome {
                role: UserRole::Contestant
            }
        ));

        // 2. Simulate Disconnect
        state.contestants.get_mut("session_2").unwrap().online = false;

        // 3. New connection, Identify
        let mut new_id = "session_2".to_string();
        let mut new_role = UserRole::Viewer;
        let msgs = OneOfFifteenWebSocket::process_message(
            IncomingMessage::Identify {
                session_id: "session_2".to_string(),
            },
            &mut state,
            &mut new_id,
            &mut new_role,
        );

        // Verify Resumption
        assert_eq!(new_role, UserRole::Contestant);
        assert!(state.contestants.get("session_2").unwrap().online);
        assert!(matches!(
            msgs[0],
            OutgoingMessage::Welcome {
                role: UserRole::Contestant
            }
        ));
    }
}
