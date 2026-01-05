use actix::prelude::*;
use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
const LLAMA_API_URL: &str = "http://localhost:8099/v1/chat/completions";

async fn generate_question_ai(age: &str, past_questions: &[String]) -> Option<Question> {
    let client = reqwest::Client::new();
    let past_q_text = if past_questions.is_empty() {
        "".to_string()
    } else {
        format!(
            "Do not repeat any of these previous questions: {:?}.",
            past_questions
        )
    };

    let prompt = format!(
        "Generate a single short trivia question suitable for a {} year old. {}. Format the output as JSON with fields 'text' and 'correct_answer'. Example: {{\"text\": \"What color is the sky?\", \"correct_answer\": \"Blue\"}}",
        age, past_q_text
    );

    let body = serde_json::json!({
        "messages": [
            { "role": "system", "content": "You are a game show host's assistant. Output valid JSON only." },
            { "role": "user", "content": prompt }
        ],
        "stream": false,
        "temperature": 0.8 // Increased variability
    });

    println!("Sending AI request to: {}", LLAMA_API_URL);
    println!(
        "Request body: {}",
        serde_json::to_string(&body).unwrap_or_default()
    );

    match client.post(LLAMA_API_URL).json(&body).send().await {
        Ok(res) => {
            println!("AI Request Status: {}", res.status());
            if let Ok(json) = res.json::<serde_json::Value>().await {
                println!("AI Response JSON: {}", json);
                if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                    // Try to parse content as JSON
                    // Sometimes models wrap in ```json ... ```
                    let clean_content = content
                        .replace("```json", "")
                        .replace("```", "")
                        .trim()
                        .to_string();
                    println!("Cleaned AI Content: {}", clean_content);
                    if let Ok(q) = serde_json::from_str::<Question>(&clean_content) {
                        return Some(q);
                    } else {
                        eprintln!("Failed to parse Question JSON: {}", clean_content);
                    }
                } else {
                    eprintln!("AI Response missing choices/message/content");
                }
            } else {
                eprintln!("Failed to parse AI response as JSON value");
            }
        }
        Err(e) => eprintln!("AI Request Failed: {}", e),
    }

    // Fallback if AI fails or returns None
    Some(Question {
        text: "What is the capital of France?".to_string(),
        correct_answer: "Paris".to_string(),
        options: None,
    })
}

async fn validate_answer_ai(question: &str, correct_answer: &str, user_answer: &str) -> bool {
    let client = reqwest::Client::new();
    let prompt = format!(
        "Question: {}\nCorrect Answer: {}\nUser Answer: {}\nIs the user answer correct? Respond with JSON: {{\"correct\": true}} or {{\"correct\": false}}.",
        question, correct_answer, user_answer
    );

    let body = serde_json::json!({
        "messages": [
             { "role": "system", "content": "You are a judge. Output valid JSON only." },
             { "role": "user", "content": prompt }
        ],
        "stream": false,
        "temperature": 0.1
    });

    match client.post(LLAMA_API_URL).json(&body).send().await {
        Ok(res) => {
            if let Ok(json) = res.json::<serde_json::Value>().await {
                if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                    let clean_content = content
                        .replace("```json", "")
                        .replace("```", "")
                        .trim()
                        .to_string();
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&clean_content) {
                        return val["correct"].as_bool().unwrap_or(false);
                    }
                }
            }
        }
        Err(e) => eprintln!("AI Validation Failed: {}", e),
    }
    // Fallback: exact match if AI fails
    user_answer
        .trim()
        .eq_ignore_ascii_case(correct_answer.trim())
}

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
    pub age: String, // Added
    pub score: i32,
    pub id: String, // WebSocket Session ID (or random UUID) - TO BE REMOVED/MIGRATED to session logic? No, keeping as connection ID for now, but need session_id for persistence.
    // Actually, let's use `id` as the persistent session_id.
    pub session_id: String,
    pub online: bool,
    pub ready: bool,
    pub lives: i32,
    pub round1_misses: i32,
    pub round1_questions: i32, // New field
    pub eliminated: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub text: String,
    pub correct_answer: String,       // Or allow AI to fuzzy match
    pub options: Option<Vec<String>>, // For multiple choice if needed? Assuming open for now as per prompt "input answer"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub presenter_id: Option<String>, // Session ID of the presenter
    pub presenter_online: bool,
    pub contestants: HashMap<String, Contestant>, // Keyed by session_id
    pub active: bool,
    pub round: Round,
    pub active_player_id: Option<String>, // Who's turn is it?
    pub player_queue: Vec<String>,        // Order of play
    pub current_question: Option<Question>,
    pub past_questions: Vec<String>, // Added to track usage
    pub timer_start: Option<u64>,    // Unix timestamp in seconds
    pub buzzer_queue: Vec<String>,
    pub last_pointer_id: Option<String>, // For Round 2 rollback
    pub decision_pending: bool,          // For Round 3 doubling/passing
}

impl GameState {
    pub fn new() -> Self {
        Self {
            presenter_id: None,
            presenter_online: false,
            contestants: HashMap::new(),
            active: true,
            round: Round::Lobby,
            active_player_id: None,
            player_queue: Vec::new(),
            current_question: None,
            past_questions: Vec::new(),
            timer_start: None,
            buzzer_queue: Vec::new(),
            last_pointer_id: None,
            decision_pending: false,
        }
    }
}

pub type GameStateHandle = Arc<Mutex<GameState>>;

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
    }, // choice: "self" or "point"
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
}

// --- Actor ---

pub struct OneOfFifteenWebSocket {
    hb: Instant,
    state: GameStateHandle,
    id: String,
    role: UserRole,
}

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

    // define new enum outside or just use a custom struct?
    // Let's use a struct Result
    fn process_message(
        msg: IncomingMessage,
        state: &mut GameState,
        connection_id: &mut String,
        connection_role: &mut UserRole,
    ) -> (Vec<OutgoingMessage>, Option<AsyncAction>) {
        let mut responses = Vec::new();
        let mut action = None;

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
                        return (responses, None);
                    }
                    if state.presenter_online {
                        responses.push(OutgoingMessage::Error {
                            message: "Presenter already exists and is online".to_string(),
                        });
                        return (responses, None);
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
            IncomingMessage::JoinContestant { name, age } => {
                // Check if I am currently Presenter? If so, resign.
                if state.presenter_id.as_ref() == Some(connection_id) {
                    state.presenter_id = None;
                    state.presenter_online = false;
                }

                // If I am already a contestant (re-join via button?), update name
                let session_id = connection_id.clone();

                let contestant = Contestant {
                    name,
                    age,
                    score: 0,
                    id: session_id.clone(),
                    session_id: session_id.clone(),
                    online: true,
                    ready: false,
                    lives: 3,
                    round1_misses: 0,
                    round1_questions: 0,
                    eliminated: false,
                };
                state.contestants.insert(session_id.clone(), contestant);
                *connection_role = UserRole::Contestant;
                responses.push(OutgoingMessage::Welcome {
                    role: UserRole::Contestant,
                });
            }
            IncomingMessage::StartGame => {
                if state.presenter_id.as_ref() == Some(connection_id) && state.round == Round::Lobby
                {
                    state.round = Round::Round1;
                    state.player_queue = state.contestants.keys().cloned().collect();
                    state.player_queue.sort();

                    // Select first player
                    if let Some(first_id) = state.player_queue.first() {
                        state.active_player_id = Some(first_id.clone());

                        // Trigger AI Question Generation
                        if let Some(contestant) = state.contestants.get(first_id) {
                            action = Some(AsyncAction::GenerateQuestion {
                                age: contestant.age.clone(),
                                past_questions: state.past_questions.clone(),
                            });
                        }

                        // Setup Timer for first player
                        use std::time::{SystemTime, UNIX_EPOCH};
                        let start = SystemTime::now();
                        state.timer_start =
                            Some(start.duration_since(UNIX_EPOCH).unwrap().as_secs());
                    }
                }
            }
            IncomingMessage::ResetGame => {
                if state.presenter_id.as_ref() == Some(connection_id) {
                    state.round = Round::Lobby;
                    state.contestants.values_mut().for_each(|c| {
                        c.score = 0;
                        c.lives = 3;
                        c.round1_misses = 0;
                        c.round1_questions = 0;
                        c.eliminated = false;
                    });
                    state.active_player_id = None;
                    state.current_question = None;
                    state.timer_start = None;
                }
            }
            IncomingMessage::GetState => {
                let snapshot = GameStateSnapshot {
                    has_presenter: state.presenter_id.is_some(),
                    presenter_online: state.presenter_online,
                    contestants: state.contestants.values().cloned().collect(),
                    round: state.round.clone(),
                    active_player_id: state.active_player_id.clone(),
                    current_question: state.current_question.clone(),
                    timer_start: state.timer_start,
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
                        round: state.round.clone(),
                        active_player_id: state.active_player_id.clone(),
                        current_question: state.current_question.clone(),
                        timer_start: state.timer_start,
                    };
                    responses.push(OutgoingMessage::StateUpdate(snapshot));
                }
            }
            IncomingMessage::SubmitAnswer { answer } => {
                // Check if active player
                if state.active_player_id.as_ref() == Some(connection_id) {
                    let mut final_answer = answer.clone();

                    // Check Timer Logic (Round 1)
                    if state.round == Round::Round1 {
                        if let Some(start_ts) = state.timer_start {
                            use std::time::{SystemTime, UNIX_EPOCH};
                            if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) {
                                if now.as_secs() > start_ts + 62 {
                                    // 60s + 2s buffer
                                    final_answer = "!!!TIMEOUT!!!".to_string();
                                }
                            }
                        }
                    }

                    if let Some(q) = &state.current_question {
                        // Trigger Async Validation
                        action = Some(AsyncAction::ValidateAnswer {
                            question: q.text.clone(),
                            correct: q.correct_answer.clone(),
                            answer: final_answer,
                            player_id: connection_id.clone(),
                        });
                        // Clear question immediately to prevent double submission?
                        // Or wait for result. Let's wait.
                    }
                }
            }
            IncomingMessage::PointToPlayer { target_id } => {
                let is_active = state.active_player_id.as_ref() == Some(connection_id);
                let is_round2 = state.round == Round::Round2;

                println!(
                    "DEBUG: PointToPlayer received. Sender: {}, Target: {}",
                    connection_id, target_id
                );
                println!("DEBUG: is_active: {}, is_round2: {}", is_active, is_round2);

                if is_active && is_round2 {
                    // Verify target
                    let target_valid = if let Some(target) = state.contestants.get(&target_id) {
                        println!(
                            "DEBUG: Target found. Eliminated: {}, Online: {}, ID: {}",
                            target.eliminated, target.online, target.id
                        );
                        !target.eliminated && target.online && &target.id != connection_id
                    } else {
                        println!("DEBUG: Target NOT found: {}", target_id);
                        false
                    };

                    if target_valid {
                        state.active_player_id = Some(target_id.clone());

                        // Generate Question for target
                        if let Some(target) = state.contestants.get(&target_id) {
                            action = Some(AsyncAction::GenerateQuestion {
                                age: target.age.clone(),
                                past_questions: state.past_questions.clone(),
                            });
                        }

                        // Start Timer
                        use std::time::{SystemTime, UNIX_EPOCH};
                        let start = SystemTime::now();
                        state.timer_start =
                            Some(start.duration_since(UNIX_EPOCH).unwrap().as_secs());
                    }
                }
            }
            IncomingMessage::BuzzIn => {
                // Round 3 only
                if state.round == Round::Round3
                    && state.active_player_id.is_none()
                    && !state.decision_pending
                {
                    // Check if player is eligible (not eliminated)
                    if let Some(c) = state.contestants.get(connection_id) {
                        if !c.eliminated && c.online {
                            state.active_player_id = Some(connection_id.clone());

                            // Generate Question Immediately
                            action = Some(AsyncAction::GenerateQuestion {
                                age: c.age.clone(),
                                past_questions: state.past_questions.clone(),
                            });

                            // Start Timer
                            use std::time::{SystemTime, UNIX_EPOCH};
                            let start = SystemTime::now();
                            state.timer_start =
                                Some(start.duration_since(UNIX_EPOCH).unwrap().as_secs());
                        }
                    }
                }
            }
            IncomingMessage::MakeDecision { choice, target_id } => {
                // Round 3 only, active player, decision pending
                let is_active = state.active_player_id.as_ref() == Some(connection_id);
                if state.round == Round::Round3 && is_active && state.decision_pending {
                    if choice == "self" {
                        // Double Down - Generate new question for SELF
                        state.decision_pending = false;
                        if let Some(c) = state.contestants.get(connection_id) {
                            action = Some(AsyncAction::GenerateQuestion {
                                age: c.age.clone(),
                                past_questions: state.past_questions.clone(),
                            });
                            // Restart Timer
                            use std::time::{SystemTime, UNIX_EPOCH};
                            let start = SystemTime::now();
                            state.timer_start =
                                Some(start.duration_since(UNIX_EPOCH).unwrap().as_secs());
                        }
                    } else if choice == "point" {
                        // Point to another
                        if let Some(tid) = target_id {
                            // Verify target
                            let target_valid = if let Some(target) = state.contestants.get(&tid) {
                                !target.eliminated && target.online && &target.id != connection_id
                            } else {
                                false
                            };

                            if target_valid {
                                state.decision_pending = false;
                                state.active_player_id = Some(tid.clone());

                                // Generate Question for target
                                if let Some(target) = state.contestants.get(&tid) {
                                    action = Some(AsyncAction::GenerateQuestion {
                                        age: target.age.clone(),
                                        past_questions: state.past_questions.clone(),
                                    });
                                }

                                // Start Timer
                                use std::time::{SystemTime, UNIX_EPOCH};
                                let start = SystemTime::now();
                                state.timer_start =
                                    Some(start.duration_since(UNIX_EPOCH).unwrap().as_secs());
                            }
                        }
                    }
                }
            }
        }
        (responses, action)
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
                println!("🔵 RAW WebSocket message received: {}", text);
                match serde_json::from_str::<IncomingMessage>(&text) {
                    Ok(input) => {
                        println!("✅ Successfully deserialized message: {:?}", input);
                        let mut state = self.state.lock().unwrap();
                        let (responses, action) =
                            Self::process_message(input, &mut state, &mut self.id, &mut self.role);

                        for msg in responses {
                            if let Ok(json) = serde_json::to_string(&msg) {
                                ctx.text(json);
                            }
                        }

                        if let Some(act) = action {
                            let state = self.state.clone();
                            match act {
                                AsyncAction::GenerateQuestion {
                                    age,
                                    past_questions,
                                } => {
                                    ctx.spawn(
                                        actix::fut::wrap_future(async move {
                                            generate_question_ai(&age, &past_questions).await
                                        })
                                        .map(
                                            move |res, _, _| {
                                                if let Some(q) = res {
                                                    let mut state = state.lock().unwrap();
                                                    state.past_questions.push(q.text.clone());
                                                    state.current_question = Some(q);
                                                }
                                            },
                                        ),
                                    );
                                }
                                AsyncAction::ValidateAnswer {
                                    question,
                                    correct,
                                    answer,
                                    player_id,
                                } => {
                                    ctx.spawn(
                                        actix::fut::wrap_future(async move {
                                            validate_answer_ai(&question, &correct, &answer).await
                                        })
                                        .map(
                                            move |is_correct,
                                                  _,
                                                  ctx: &mut ws::WebsocketContext<
                                                OneOfFifteenWebSocket,
                                            >| {
                                                let state_arc = state.clone();
                                                let mut state = state.lock().unwrap();

                                                // Update score/lives and broadcast
                                                let msgs =
                                                    OneOfFifteenWebSocket::handle_validate_answer(
                                                        &mut state, is_correct, player_id, ctx,
                                                        state_arc,
                                                    );

                                                for msg in msgs {
                                                    if let Ok(json) = serde_json::to_string(&msg) {
                                                        ctx.text(json);
                                                    }
                                                }
                                            },
                                        ),
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!(
                            "ERROR: Failed to parse IncomingMessage: {}. Text: {}",
                            e, text
                        );
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

impl OneOfFifteenWebSocket {
    fn handle_validate_answer(
        state: &mut GameState,
        is_correct: bool,
        player_id: String,
        ctx: &mut ws::WebsocketContext<Self>,
        state_arc: Arc<Mutex<GameState>>,
    ) -> Vec<OutgoingMessage> {
        let round = state.round.clone();

        if let Some(c) = state.contestants.get_mut(&player_id) {
            if is_correct {
                c.score += 10;
            } else if round == Round::Round1 {
                c.round1_misses += 1;
                c.lives -= 1;
                if c.round1_misses >= 2 {
                    c.eliminated = true;
                }
            } else {
                c.lives -= 1;
            }
        }

        // Reset question & timer
        state.current_question = None;
        state.timer_start = None;

        // Round 2 Survivor Check
        if round == Round::Round2 {
            let active_count = state.contestants.values().filter(|c| !c.eliminated).count();

            if active_count <= 3 {
                state.round = Round::Round3;
                state.active_player_id = None;
                for c in state.contestants.values_mut() {
                    if !c.eliminated {
                        c.lives = 3;
                    }
                }
                let snapshot = GameStateSnapshot {
                    has_presenter: state.presenter_id.is_some(),
                    presenter_online: state.presenter_online,
                    contestants: state.contestants.values().cloned().collect(),
                    round: state.round.clone(),
                    active_player_id: state.active_player_id.clone(),
                    current_question: state.current_question.clone(),
                    timer_start: state.timer_start,
                };
                return vec![OutgoingMessage::StateUpdate(snapshot)];
            }

            if is_correct {
                state.last_pointer_id = Some(player_id.clone());
                state.active_player_id = Some(player_id.clone());
                let snapshot = GameStateSnapshot {
                    has_presenter: state.presenter_id.is_some(),
                    presenter_online: state.presenter_online,
                    contestants: state.contestants.values().cloned().collect(),
                    round: state.round.clone(),
                    active_player_id: state.active_player_id.clone(),
                    current_question: state.current_question.clone(),
                    timer_start: state.timer_start,
                };
                return vec![OutgoingMessage::StateUpdate(snapshot)];
            } else if let Some(prev) = &state.last_pointer_id {
                if let Some(prev_c) = state.contestants.get(prev) {
                    if !prev_c.eliminated && prev_c.online {
                        state.active_player_id = Some(prev.clone());
                    }
                }
            }
            let snapshot = GameStateSnapshot {
                has_presenter: state.presenter_id.is_some(),
                presenter_online: state.presenter_online,
                contestants: state.contestants.values().cloned().collect(),
                round: state.round.clone(),
                active_player_id: state.active_player_id.clone(),
                current_question: state.current_question.clone(),
                timer_start: state.timer_start,
            };
            return vec![OutgoingMessage::StateUpdate(snapshot)];
        }

        // Round 3 Logic
        if round == Round::Round3 {
            if is_correct {
                state.decision_pending = true;
                state.current_question = None;
                state.timer_start = None;
            } else {
                state.active_player_id = None;
                state.current_question = None;
                state.timer_start = None;
            }
            let active_count = state.contestants.values().filter(|c| !c.eliminated).count();
            if active_count <= 1 {
                state.round = Round::Finished;
                state.active_player_id = None;
            }
            let snapshot = GameStateSnapshot {
                has_presenter: state.presenter_id.is_some(),
                presenter_online: state.presenter_online,
                contestants: state.contestants.values().cloned().collect(),
                round: state.round.clone(),
                active_player_id: state.active_player_id.clone(),
                current_question: state.current_question.clone(),
                timer_start: state.timer_start,
            };
            return vec![OutgoingMessage::StateUpdate(snapshot)];
        }

        // Round 1 Logic
        if round == Round::Round1 {
            if let Some(c) = state.contestants.get_mut(&player_id) {
                c.round1_questions += 1;
            }
        }

        // Rotation Logic
        let player_queue = state.player_queue.clone();
        let mut next_player_info = None;

        if let Some(idx) = player_queue.iter().position(|id| id == &player_id) {
            let mut next_idx = (idx + 1) % player_queue.len();
            for _ in 0..player_queue.len() {
                if let Some(next_id) = player_queue.get(next_idx) {
                    if let Some(next_c) = state.contestants.get(next_id) {
                        let is_eligible = !next_c.eliminated && next_c.online;
                        let round1_done = round == Round::Round1 && next_c.round1_questions >= 2;
                        if is_eligible && !round1_done {
                            next_player_info = Some((next_id.clone(), next_c.age.clone()));
                            break;
                        }
                    }
                }
                next_idx = (next_idx + 1) % player_queue.len();
            }
        }

        if let Some((next_id, age)) = next_player_info {
            state.active_player_id = Some(next_id);

            // Timer
            use std::time::{SystemTime, UNIX_EPOCH};
            let start = SystemTime::now();
            state.timer_start = Some(start.duration_since(UNIX_EPOCH).unwrap().as_secs());

            // Generate Question
            let inner_state = state_arc.clone();
            let past_questions = state.past_questions.clone();
            ctx.spawn(
                actix::fut::wrap_future(async move {
                    generate_question_ai(&age, &past_questions).await
                })
                .map(move |res, _, _| {
                    if let Some(q) = res {
                        let mut state = inner_state.lock().unwrap();
                        state.past_questions.push(q.text.clone());
                        state.current_question = Some(q);
                    }
                }),
            );
            // Question generation spawned, broadcast state
            let snapshot = GameStateSnapshot {
                has_presenter: state.presenter_id.is_some(),
                presenter_online: state.presenter_online,
                contestants: state.contestants.values().cloned().collect(),
                round: state.round.clone(),
                active_player_id: state.active_player_id.clone(),
                current_question: state.current_question.clone(),
                timer_start: state.timer_start,
            };
            vec![OutgoingMessage::StateUpdate(snapshot)]
        } else {
            // No next player found in rotation
            if round == Round::Round1 {
                state.round = Round::Round2;
                // Start Round 2 with first eligible
                for id in &player_queue {
                    if let Some(c) = state.contestants.get(id) {
                        if !c.eliminated && c.online {
                            next_player_info = Some((id.clone(), c.age.clone()));
                            break;
                        }
                    }
                }
                if let Some((next_id, age)) = next_player_info {
                    state.active_player_id = Some(next_id);
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let start = SystemTime::now();
                    state.timer_start = Some(start.duration_since(UNIX_EPOCH).unwrap().as_secs());

                    let inner_state = state_arc.clone();
                    let past_questions = state.past_questions.clone();
                    ctx.spawn(
                        actix::fut::wrap_future(async move {
                            generate_question_ai(&age, &past_questions).await
                        })
                        .map(move |res, _, _| {
                            if let Some(q) = res {
                                let mut state = inner_state.lock().unwrap();
                                state.past_questions.push(q.text.clone());
                                state.current_question = Some(q);
                            }
                        }),
                    );
                    let snapshot = GameStateSnapshot {
                        has_presenter: state.presenter_id.is_some(),
                        presenter_online: state.presenter_online,
                        contestants: state.contestants.values().cloned().collect(),
                        round: state.round.clone(),
                        active_player_id: state.active_player_id.clone(),
                        current_question: state.current_question.clone(),
                        timer_start: state.timer_start,
                    };
                    vec![OutgoingMessage::StateUpdate(snapshot)]
                } else {
                    state.round = Round::Finished;
                    state.active_player_id = None;
                    let snapshot = GameStateSnapshot {
                        has_presenter: state.presenter_id.is_some(),
                        presenter_online: state.presenter_online,
                        contestants: state.contestants.values().cloned().collect(),
                        round: state.round.clone(),
                        active_player_id: state.active_player_id.clone(),
                        current_question: state.current_question.clone(),
                        timer_start: state.timer_start,
                    };
                    vec![OutgoingMessage::StateUpdate(snapshot)]
                }
            } else {
                state.active_player_id = None;
                let snapshot = GameStateSnapshot {
                    has_presenter: state.presenter_id.is_some(),
                    presenter_online: state.presenter_online,
                    contestants: state.contestants.values().cloned().collect(),
                    round: state.round.clone(),
                    active_player_id: state.active_player_id.clone(),
                    current_question: state.current_question.clone(),
                    timer_start: state.timer_start,
                };
                vec![OutgoingMessage::StateUpdate(snapshot)]
            }
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
        let (msgs, _) = OneOfFifteenWebSocket::process_message(
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
        let (msgs, _) = OneOfFifteenWebSocket::process_message(
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
        let (msgs, _) = OneOfFifteenWebSocket::process_message(
            IncomingMessage::JoinContestant {
                name: "Alice".to_string(),
                age: "25".to_string(),
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
        let (msgs, _) = OneOfFifteenWebSocket::process_message(
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
    #[test]
    fn test_submit_answer_triggers_validation() {
        let mut state = GameState::new();
        let mut id = "session_3".to_string();
        let mut role = UserRole::Contestant;

        // Setup state: Contestant, Round1, Question Active, Active Player
        let contestant = Contestant {
            name: "Bob".to_string(),
            age: "30".to_string(),
            score: 0,
            id: id.clone(),
            session_id: id.clone(),
            online: true,
            ready: true,
            lives: 3,
            round1_misses: 0,
            round1_questions: 0,
            eliminated: false,
        };
        state.contestants.insert(id.clone(), contestant);
        state.round = Round::Round1;
        state.active_player_id = Some(id.clone());
        state.current_question = Some(Question {
            text: "Q1".to_string(),
            correct_answer: "A1".to_string(),
            options: None,
        });

        // Submit Answer
        let (_, action) = OneOfFifteenWebSocket::process_message(
            IncomingMessage::SubmitAnswer {
                answer: "A1".to_string(),
            },
            &mut state,
            &mut id,
            &mut role,
        );

        // Verify AsyncAction Triggered
        if let Some(AsyncAction::ValidateAnswer {
            question,
            correct,
            answer,
            player_id,
        }) = action
        {
            assert_eq!(question, "Q1");
            assert_eq!(correct, "A1");
            assert_eq!(answer, "A1");
            assert_eq!(player_id, "session_3");
        } else {
            panic!("Expected ValidateAnswer action");
        }
    }
    #[test]
    fn test_round3_buzzer_logic() {
        let mut state = GameState::new();
        let mut id = "session_4".to_string();
        let mut role = UserRole::Contestant;

        // Setup: Round 3, No Active Player
        let contestant = Contestant {
            name: "Dave".to_string(),
            age: "40".to_string(),
            score: 0,
            id: id.clone(),
            session_id: id.clone(),
            online: true,
            ready: true,
            lives: 3,
            round1_misses: 0,
            round1_questions: 0,
            eliminated: false,
        };
        state.contestants.insert(id.clone(), contestant);
        state.round = Round::Round3;
        state.active_player_id = None;
        state.decision_pending = false;

        // Buzz In
        let (_, action) = OneOfFifteenWebSocket::process_message(
            IncomingMessage::BuzzIn,
            &mut state,
            &mut id,
            &mut role,
        );

        // Verify Active Player Set & Question Generation Triggered
        assert_eq!(state.active_player_id, Some(id.clone()));
        assert!(matches!(action, Some(AsyncAction::GenerateQuestion { .. })));
    }

    #[test]
    fn test_round3_decision_logic() {
        let mut state = GameState::new();
        let mut id = "session_5".to_string();
        let mut role = UserRole::Contestant;

        // Setup: Round 3, Active Player, Decision Pending
        let contestant = Contestant {
            name: "Eve".to_string(),
            age: "50".to_string(),
            score: 0,
            id: id.clone(),
            session_id: id.clone(),
            online: true,
            ready: true,
            lives: 3,
            round1_misses: 0,
            round1_questions: 0,
            eliminated: false,
        };
        state.contestants.insert(id.clone(), contestant);
        state.round = Round::Round3;
        state.active_player_id = Some(id.clone());
        state.decision_pending = true;

        // Make Decision (Self)
        let (_, action) = OneOfFifteenWebSocket::process_message(
            IncomingMessage::MakeDecision {
                choice: "self".to_string(),
                target_id: None,
            },
            &mut state,
            &mut id,
            &mut role,
        );

        // Verify: Decision Pending False, Active Player Same, Question Generation Triggered
        assert!(!state.decision_pending);
        assert_eq!(state.active_player_id, Some(id.clone()));
        assert!(matches!(action, Some(AsyncAction::GenerateQuestion { .. })));
    }
}
