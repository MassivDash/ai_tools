use actix::prelude::*;
use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// Import from our modules
use super::ai::{generate_question_ai, validate_answer_ai};
use super::rounds;
use super::state::GameStateHandle;
use super::types::*;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

// --- WebSocket Actor ---

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
            id: uuid::Uuid::new_v4().to_string(),
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
    ) -> (Vec<OutgoingMessage>, Option<AsyncAction>) {
        let mut responses = Vec::new();
        let mut action = None;

        match msg {
            IncomingMessage::Identify { session_id } => {
                *connection_id = session_id.clone();

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
                    responses.push(OutgoingMessage::Error {
                        message: "Session not found".to_string(),
                    });
                }
            }
            IncomingMessage::JoinPresenter => {
                if state.contestants.contains_key(connection_id) {
                    state.contestants.remove(connection_id);
                }

                if let Some(pid) = &state.presenter_id {
                    if pid == connection_id {
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
                if state.presenter_id.as_ref() == Some(connection_id) {
                    state.presenter_id = None;
                    state.presenter_online = false;
                }

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
                state.contestants.insert(session_id, contestant);
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

                    if let Some(first_id) = state.player_queue.first() {
                        state.active_player_id = Some(first_id.clone());

                        if let Some(contestant) = state.contestants.get(first_id) {
                            action = Some(AsyncAction::GenerateQuestion {
                                age: contestant.age.clone(),
                                past_questions: state.past_questions.clone(),
                            });
                        }

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
                responses.push(rounds::common::create_state_update(state));
            }
            IncomingMessage::ToggleReady => {
                if let Some(c) = state.contestants.get_mut(connection_id) {
                    c.ready = !c.ready;
                    responses.push(rounds::common::create_state_update(state));
                }
            }
            IncomingMessage::SubmitAnswer { answer } => {
                if state.active_player_id.as_ref() == Some(connection_id) {
                    let mut final_answer = answer.clone();

                    if state.round == Round::Round1 {
                        if let Some(start_ts) = state.timer_start {
                            if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) {
                                if now.as_secs() > start_ts + 62 {
                                    final_answer = "!!!TIMEOUT!!!".to_string();
                                }
                            }
                        }
                    }

                    if let Some(q) = &state.current_question {
                        action = Some(AsyncAction::ValidateAnswer {
                            question: q.text.clone(),
                            correct: q.correct_answer.clone(),
                            answer: final_answer,
                            player_id: connection_id.clone(),
                        });
                    }
                }
            }
            IncomingMessage::PointToPlayer { target_id } => {
                if state.round == Round::Round2 {
                    // Delegate to round2 module for validation and state update
                    let msgs = rounds::round2::handle_point_to_player(state, &target_id);
                    responses.extend(msgs);

                    // Generate question for the targeted player
                    if let Some(target) = state.contestants.get(&target_id) {
                        action = Some(AsyncAction::GenerateQuestion {
                            age: target.age.clone(),
                            past_questions: state.past_questions.clone(),
                        });

                        let start = SystemTime::now();
                        state.timer_start =
                            Some(start.duration_since(UNIX_EPOCH).unwrap().as_secs());
                    }
                }
            }
            IncomingMessage::BuzzIn => {
                if state.round == Round::Round3
                    && state.active_player_id.is_none()
                    && !state.decision_pending
                {
                    // Delegate to round3 module for validation
                    let msgs = rounds::round3::handle_buzz_in(state, connection_id);
                    responses.extend(msgs);

                    // Generate question only if player is valid
                    if let Some(c) = state.contestants.get(connection_id) {
                        if !c.eliminated && c.online {
                            action = Some(AsyncAction::GenerateQuestion {
                                age: c.age.clone(),
                                past_questions: state.past_questions.clone(),
                            });

                            let start = SystemTime::now();
                            state.timer_start =
                                Some(start.duration_since(UNIX_EPOCH).unwrap().as_secs());
                        }
                    }
                }
            }
            IncomingMessage::MakeDecision { choice, target_id } => {
                let is_active = state.active_player_id.as_ref() == Some(connection_id);

                if state.round == Round::Round3 && is_active && state.decision_pending {
                    // Delegate to round3 module for score doubling and state management
                    let msgs = rounds::round3::handle_correct_answer_decision(
                        state,
                        connection_id,
                        &choice,
                        target_id.clone(),
                    );
                    responses.extend(msgs);

                    // Generate next question based on the decision
                    let next_player = state.active_player_id.clone();
                    if let Some(player_id) = next_player {
                        if let Some(player) = state.contestants.get(&player_id) {
                            action = Some(AsyncAction::GenerateQuestion {
                                age: player.age.clone(),
                                past_questions: state.past_questions.clone(),
                            });

                            let start = SystemTime::now();
                            state.timer_start =
                                Some(start.duration_since(UNIX_EPOCH).unwrap().as_secs());
                        }
                    }
                }
            }
        }
        (responses, action)
    }

    fn spawn_question_generation(
        ctx: &mut ws::WebsocketContext<Self>,
        state: GameStateHandle,
        age: String,
        past_questions: Vec<String>,
    ) {
        ctx.spawn(
            actix::fut::wrap_future(
                async move { generate_question_ai(&age, &past_questions).await },
            )
            .map(
                move |res, _, ctx: &mut ws::WebsocketContext<OneOfFifteenWebSocket>| {
                    if let Some(q) = res {
                        let mut state = state.lock().unwrap();
                        state.past_questions.push(q.text.clone());
                        state.current_question = Some(q);

                        let snapshot = rounds::common::create_state_snapshot(&state);
                        let msg = OutgoingMessage::StateUpdate(snapshot);
                        if let Ok(json) = serde_json::to_string(&msg) {
                            ctx.text(json);
                        }
                    }
                },
            ),
        );
    }

    fn handle_validate_answer(
        state: &mut GameState,
        is_correct: bool,
        player_id: String,
    ) -> (Vec<OutgoingMessage>, Option<AsyncAction>) {
        let round = state.round.clone();

        // Delegate to round-specific logic
        match round {
            Round::Round1 => {
                let msgs = if is_correct {
                    rounds::round1::handle_correct_answer(state, &player_id)
                } else {
                    rounds::round1::handle_wrong_answer(state, &player_id)
                };

                // Check if there's a next player who needs a question
                let action = if let Some(next_player_id) = &state.active_player_id {
                    if let Some(contestant) = state.contestants.get(next_player_id) {
                        Some(AsyncAction::GenerateQuestion {
                            age: contestant.age.clone(),
                            past_questions: state.past_questions.clone(),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                };

                (msgs, action)
            }
            Round::Round2 => {
                let msgs = if is_correct {
                    rounds::round2::handle_correct_answer(state, &player_id)
                } else {
                    rounds::round2::handle_wrong_answer(state, &player_id)
                };

                // Check if there's a next player who needs a question
                let action = if let Some(next_player_id) = &state.active_player_id {
                    if let Some(contestant) = state.contestants.get(next_player_id) {
                        Some(AsyncAction::GenerateQuestion {
                            age: contestant.age.clone(),
                            past_questions: state.past_questions.clone(),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                };

                (msgs, action)
            }
            Round::Round3 => {
                if is_correct {
                    // Check if player has reached 30 points AFTER this answer
                    let player_score = state
                        .contestants
                        .get(&player_id)
                        .map(|c| c.score)
                        .unwrap_or(0);

                    if player_score >= 30 {
                        // Player has 30+ points - enable decision mechanic
                        state.decision_pending = true;
                        rounds::common::reset_question_state(state);
                        (vec![rounds::common::create_state_update(state)], None)
                    } else {
                        // Below 30 points - just award points, no decision
                        rounds::common::award_points(state, &player_id, 10);
                        rounds::common::reset_question_state(state);
                        state.active_player_id = None; // Wait for next buzz
                        (vec![rounds::common::create_state_update(state)], None)
                    }
                } else {
                    let msgs = rounds::round3::handle_wrong_answer(state, &player_id);
                    // No next question in Round 3 after wrong answer - wait for buzz-in
                    (msgs, None)
                }
            }
            _ => (vec![rounds::common::create_state_update(state)], None),
        }
    }
}

impl Actor for OneOfFifteenWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> actix::Running {
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
            Ok(ws::Message::Text(text)) => match serde_json::from_str::<IncomingMessage>(&text) {
                Ok(input) => {
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
                                Self::spawn_question_generation(ctx, state, age, past_questions);
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
                                            let state_clone = state.clone();
                                            let mut state_lock = state.lock().unwrap();

                                            let (msgs, next_action) =
                                                OneOfFifteenWebSocket::handle_validate_answer(
                                                    &mut state_lock,
                                                    is_correct,
                                                    player_id,
                                                );

                                            for msg in msgs {
                                                if let Ok(json) = serde_json::to_string(&msg) {
                                                    ctx.text(json);
                                                }
                                            }

                                            // Generate question for next player if needed
                                            if let Some(AsyncAction::GenerateQuestion {
                                                age,
                                                past_questions,
                                            }) = next_action
                                            {
                                                drop(state_lock); // Release lock before spawning
                                                Self::spawn_question_generation(
                                                    ctx,
                                                    state_clone,
                                                    age,
                                                    past_questions,
                                                );
                                            }
                                        },
                                    ),
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to parse IncomingMessage: {}. Text: {}", e, text);
                }
            },
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
