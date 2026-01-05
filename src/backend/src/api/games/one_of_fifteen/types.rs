use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// --- Enums ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Presenter,
    Contestant,
    Viewer, // Default
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Round {
    Lobby,
    Round1,
    Round2,
    Round3,
    Finished,
}

// --- Structs ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contestant {
    pub name: String,
    pub age: String,
    pub score: i32,
    pub id: String,
    pub session_id: String,
    pub online: bool,
    pub ready: bool,
    pub lives: i32,
    pub round1_misses: i32,
    pub round1_questions: i32,
    pub eliminated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub text: String,
    pub correct_answer: String,
    pub options: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub presenter_id: Option<String>,
    pub presenter_online: bool,
    pub contestants: HashMap<String, Contestant>,
    pub active: bool,
    pub round: Round,
    pub active_player_id: Option<String>,
    pub player_queue: Vec<String>,
    pub current_question: Option<Question>,
    pub past_questions: Vec<String>,
    pub timer_start: Option<u64>,
    pub buzzer_queue: Vec<String>,
    pub last_pointer_id: Option<String>,
    pub decision_pending: bool,
}

// --- WebSocket Messages (Incoming from Client) ---

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IncomingMessage {
    Identify {
        session_id: String,
    },
    JoinPresenter,
    JoinContestant {
        name: String,
        age: String,
    },
    StartGame,
    ResetGame,
    GetState,
    ToggleReady,
    SubmitAnswer {
        answer: String,
    },
    PointToPlayer {
        target_id: String,
    },
    BuzzIn,
    MakeDecision {
        choice: String,
        target_id: Option<String>,
    },
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
    pub presenter_online: bool,
    pub contestants: Vec<Contestant>,
    pub round: Round,
    pub active_player_id: Option<String>,
    pub current_question: Option<Question>,
    pub timer_start: Option<u64>,
    pub decision_pending: bool,
}

// --- Async Action Types ---

#[derive(Debug)]
pub enum AsyncAction {
    GenerateQuestion {
        age: String,
        past_questions: Vec<String>,
    },
    ValidateAnswer {
        question: String,
        correct: String,
        answer: String,
        player_id: String,
    },
}
