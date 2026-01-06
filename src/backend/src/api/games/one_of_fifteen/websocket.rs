use actix::prelude::*;
use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// Import from our modules
use super::ai::{generate_question_ai, validate_answer_ai};
use super::rounds;
use super::state::GameStateHandle;
use super::types::*;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

// --- Message Types ---

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct BroadcastingMessage(pub OutgoingMessage);

// --- WebSocket Actor ---

pub struct OneOfFifteenWebSocket {
    hb: Instant,
    state: GameStateHandle,
    id: String,
    role: UserRole,
    broadcaster: super::BroadcastHandle,
    ai_api_url: String,
}

impl OneOfFifteenWebSocket {
    pub fn new(
        state: GameStateHandle,
        broadcaster: super::BroadcastHandle,
        ai_api_url: String,
    ) -> Self {
        Self {
            hb: Instant::now(),
            state,
            id: uuid::Uuid::new_v4().to_string(),
            role: UserRole::Viewer,
            broadcaster,
            ai_api_url,
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

                        // Timer will be started when question is generated
                        // state.timer_start =
                        //     Some(start.duration_since(UNIX_EPOCH).unwrap().as_secs());
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
                        c.ready = false; // Reset ready state
                    });
                    state.active_player_id = None;
                    state.current_question = None;
                    state.timer_start = None;
                    state.decision_pending = false;
                    state.past_questions = vec![];
                    state.buzzer_queue = vec![];
                    state.last_pointer_id = None;
                    state.player_queue = vec![]; // Clear player queue
                    state.active = true; // Ensure game is active
                    state.round3_exclusive = false; // Reset Exclusive Mode
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

                    if state.round == Round::Round1
                        && rounds::common::is_timed_out(state.timer_start, 62)
                    {
                        final_answer = "!!!TIMEOUT!!!".to_string();
                    }

                    if final_answer == "!!!TIMEOUT!!!" {
                        let (msgs, next_action) =
                            Self::handle_timeout(state, connection_id.clone());
                        responses.extend(msgs);
                        action = next_action;
                    } else if let Some(q) = &state.current_question {
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
                let is_active = state.active_player_id.as_ref() == Some(connection_id);
                if state.round == Round::Round2 && is_active {
                    // Delegate to round2 module for validation and state update
                    let msgs = rounds::round2::handle_point_to_player(state, &target_id);
                    responses.extend(msgs);

                    // Generate question for the targeted player
                    if let Some(target) = state.contestants.get(&target_id) {
                        action = Some(AsyncAction::GenerateQuestion {
                            age: target.age.clone(),
                            past_questions: state.past_questions.clone(),
                        });

                        // Timer started in generation
                        // let start = SystemTime::now();
                        // state.timer_start =
                        //     Some(start.duration_since(UNIX_EPOCH).unwrap().as_secs());
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

                    // BuzzIn only locks the player. Question is already persistent.
                    // No new question generation needed here.
                    // action is already None from outer scope
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
                    // Use random active player for age context since it is open floor
                    // Generate next question based on the decision
                    // Use the ACTIVE PLAYER (who was set in handle_correct_answer_decision) for context
                    // If it was "Self", active_player_id is current player.
                    // If it was "Point", active_player_id is target.
                    if let Some(target_player_id) = &state.active_player_id {
                        if let Some(player) = state.contestants.get(target_player_id) {
                            action = Some(AsyncAction::GenerateQuestion {
                                age: player.age.clone(),
                                past_questions: state.past_questions.clone(),
                            });
                        }
                    } else {
                        // If for some reason no active player, fallback to random (shouldn't happen in logic)
                        if let Some(random_id) =
                            crate::api::games::one_of_fifteen::player_selection::select_random_active(state)
                        {
                            if let Some(player) = state.contestants.get(&random_id) {
                                action = Some(AsyncAction::GenerateQuestion {
                                    age: player.age.clone(),
                                    past_questions: state.past_questions.clone(),
                                });
                            }
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
        broadcaster: super::BroadcastHandle,
        ai_api_url: String,
    ) {
        ctx.spawn(
            actix::fut::wrap_future(async move {
                generate_question_ai(&ai_api_url, &age, &past_questions).await
            })
            .map(
                move |res, _, _ctx: &mut ws::WebsocketContext<OneOfFifteenWebSocket>| {
                    if let Some(q) = res {
                        let mut state = state.lock().unwrap();
                        state.past_questions.push(q.text.clone());
                        state.current_question = Some(q);

                        // Start timer here!
                        let start = SystemTime::now();
                        state.timer_start =
                            Some(start.duration_since(UNIX_EPOCH).unwrap().as_secs());

                        let snapshot = rounds::common::create_state_snapshot(&state);
                        let msg = OutgoingMessage::StateUpdate(snapshot);

                        // Broadcast!
                        let b = broadcaster.lock().unwrap();
                        for recip in b.iter() {
                            recip.do_send(BroadcastingMessage(msg.clone()));
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
                    // Only generate if we are still in Round 1.
                    // If we transitioned to Round 2, the active player (random starter)
                    // should POINT, not Answer immediately.
                    if state.round == Round::Round1 {
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

                // In Round 2, we NEVER automatically generate a question after an answer.
                // The next step is always for the active player (either the retained pointer
                // or the new random starter) to POINT to someone.
                // That PointToPlayer message will trigger the question generation.
                // Check if we transitioned to Round 3
                let action = if state.round == Round::Round3 {
                    // Generate first question for Round 3
                    if let Some(random_id) =
                        crate::api::games::one_of_fifteen::player_selection::select_random_active(
                            state,
                        )
                    {
                        if let Some(player) = state.contestants.get(&random_id) {
                            Some(AsyncAction::GenerateQuestion {
                                age: player.age.clone(),
                                past_questions: state.past_questions.clone(),
                            })
                        } else {
                            None
                        }
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
                    // Check score or if already in exclusive mode
                    let player_score = state
                        .contestants
                        .get(&player_id)
                        .map(|c| c.score)
                        .unwrap_or(0);

                    // Entry Condition: Score >= 30
                    // Sustain Condition: Already in Exclusive Mode
                    if player_score >= 30 || state.round3_exclusive {
                        // STAY IN / ENTER EXCLUSIVE MODE
                        state.round3_exclusive = true;
                        state.decision_pending = true;
                        rounds::common::reset_question_state(state);

                        // STOP question generation logic here.
                        // We must wait for the player to make a Decision (Point/Self).
                        // Do NOT clear active_player_id. It stays with the correct answerer.
                        (vec![rounds::common::create_state_update(state)], None)
                    } else {
                        // Below 30 points AND not in exclusive mode
                        rounds::common::award_points(state, &player_id, 10);
                        rounds::common::reset_question_state(state);
                        state.active_player_id = None; // Wait for next buzz

                        // Generate next question immediately (Group)
                        let action = if let Some(random_id) =
                            crate::api::games::one_of_fifteen::player_selection::select_random_active(state)
                        {
                            if let Some(player) = state.contestants.get(&random_id) {
                                Some(AsyncAction::GenerateQuestion {
                                    age: player.age.clone(),
                                    past_questions: state.past_questions.clone(),
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        (vec![rounds::common::create_state_update(state)], action)
                    }
                } else {
                    let msgs = rounds::round3::handle_wrong_answer(state, &player_id);

                    // WRONG ANSWER -> BREAK CHAIN
                    state.round3_exclusive = false;
                    state.decision_pending = false;

                    // Generate next question immediately (Group) - Return to Buzzing
                    let action = if let Some(random_id) =
                        crate::api::games::one_of_fifteen::player_selection::select_random_active(
                            state,
                        ) {
                        if let Some(player) = state.contestants.get(&random_id) {
                            Some(AsyncAction::GenerateQuestion {
                                age: player.age.clone(),
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
            }
            _ => (vec![rounds::common::create_state_update(state)], None),
        }
    }

    fn handle_timeout(
        state: &mut GameState,
        player_id: String,
    ) -> (Vec<OutgoingMessage>, Option<AsyncAction>) {
        let round = state.round.clone();

        match round {
            Round::Round1 => {
                let msgs = rounds::round1::handle_timeout(state, &player_id);

                // Check for next player similar to handle_validate_answer
                let action = if let Some(next_player_id) = &state.active_player_id {
                    if state.round == Round::Round1 {
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
                    }
                } else {
                    None
                };
                (msgs, action)
            }
            // For other rounds, fall back to logic treating timeout as wrong answer if not specified
            Round::Round2 => {
                let msgs = rounds::round2::handle_wrong_answer(state, &player_id);
                (msgs, None)
            }
            Round::Round3 => {
                let msgs = rounds::round3::handle_wrong_answer(state, &player_id);
                (msgs, None)
            }
            _ => (vec![rounds::common::create_state_update(state)], None),
        }
    }
}

impl Actor for OneOfFifteenWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
        // Register self in broadcaster
        let mut b = self.broadcaster.lock().unwrap();
        b.push(ctx.address().recipient());
    }

    fn stopping(&mut self, _: &mut Self::Context) -> actix::Running {
        let mut state = self.state.lock().unwrap();
        let mut update_needed = false;

        match self.role {
            UserRole::Presenter => {
                if state.presenter_id.as_ref() == Some(&self.id) {
                    state.presenter_online = false;
                    state.presenter_id = None;
                    update_needed = true;
                }
            }
            UserRole::Contestant => {
                if state.contestants.remove(&self.id).is_some() {
                    update_needed = true;
                }
                // Also remove from queue if present
                if let Some(pos) = state.player_queue.iter().position(|x| x == &self.id) {
                    state.player_queue.remove(pos);
                }
                // If active player left, clear active player
                if state.active_player_id.as_ref() == Some(&self.id) {
                    state.active_player_id = None;
                    state.current_question = None; // Reset current question if active player leaves
                    state.timer_start = None;
                }
            }
            _ => {}
        }

        if update_needed {
            let snapshot = rounds::common::create_state_snapshot(&state);
            let msg = OutgoingMessage::StateUpdate(snapshot);
            // Broadcast the departure
            let broadcaster = self.broadcaster.lock().unwrap();
            for recip in broadcaster.iter() {
                recip.do_send(BroadcastingMessage(msg.clone()));
            }
        }

        actix::Running::Stop
    }
}

impl Handler<BroadcastingMessage> for OneOfFifteenWebSocket {
    type Result = ();

    fn handle(&mut self, msg: BroadcastingMessage, ctx: &mut Self::Context) {
        if let Ok(json) = serde_json::to_string(&msg.0) {
            ctx.text(json);
        }
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
                    let ai_api_url = self.ai_api_url.clone();
                    let mut state = self.state.lock().unwrap();
                    let (responses, action) =
                        Self::process_message(input, &mut state, &mut self.id, &mut self.role);

                    // Broadcast logic:
                    // Only broadcast if the message is a StateUpdate (which implies shared state change)
                    // The Error and Welcome messages are private to the connection.
                    let broadcaster = self.broadcaster.lock().unwrap();

                    for msg in responses {
                        match msg {
                            OutgoingMessage::StateUpdate(_) => {
                                // Broadcast to all
                                for recip in broadcaster.iter() {
                                    recip.do_send(BroadcastingMessage(msg.clone()));
                                }
                            }
                            _ => {
                                // Send only to self
                                if let Ok(json) = serde_json::to_string(&msg) {
                                    ctx.text(json);
                                }
                            }
                        }
                    }

                    if let Some(act) = action {
                        let state = self.state.clone();
                        let broadcaster_clone = self.broadcaster.clone(); // Pass for async usage if needed?
                                                                          // Actually spawn_question_generation needs to broadcast too?
                                                                          // Yes!
                        match act {
                            AsyncAction::GenerateQuestion {
                                age,
                                past_questions,
                            } => {
                                Self::spawn_question_generation(
                                    ctx,
                                    state,
                                    age,
                                    past_questions,
                                    broadcaster_clone,
                                    ai_api_url.clone(),
                                );
                            }
                            AsyncAction::ValidateAnswer {
                                question,
                                correct,
                                answer,
                                player_id,
                            } => {
                                let api_url = ai_api_url.clone();
                                ctx.spawn(
                                    actix::fut::wrap_future(async move {
                                        validate_answer_ai(&api_url, &question, &correct, &answer)
                                            .await
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

                                            let broadcaster = broadcaster_clone.lock().unwrap();

                                            for msg in msgs {
                                                match msg {
                                                    OutgoingMessage::StateUpdate(_) => {
                                                        for recip in broadcaster.iter() {
                                                            recip.do_send(BroadcastingMessage(
                                                                msg.clone(),
                                                            ));
                                                        }
                                                    }
                                                    _ => {
                                                        if let Ok(json) =
                                                            serde_json::to_string(&msg)
                                                        {
                                                            ctx.text(json);
                                                        }
                                                    }
                                                }
                                            }

                                            // Generate question for next player if needed
                                            if let Some(AsyncAction::GenerateQuestion {
                                                age,
                                                past_questions,
                                            }) = next_action
                                            {
                                                drop(state_lock); // Release lock
                                                drop(broadcaster);
                                                Self::spawn_question_generation(
                                                    ctx,
                                                    state_clone,
                                                    age,
                                                    past_questions,
                                                    broadcaster_clone.clone(),
                                                    ai_api_url.clone(),
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
    broadcaster: web::Data<super::BroadcastHandle>,
    llama_config: web::Data<Arc<std::sync::Mutex<crate::api::llama_server::types::Config>>>,
) -> Result<HttpResponse, Error> {
    let (host, port) = {
        let config = llama_config.lock().unwrap();
        (
            config
                .host
                .clone()
                .unwrap_or_else(|| "127.0.0.1".to_string()),
            config.port.unwrap_or(8090),
        )
    };

    // Normalize host (handle 0.0.0.0 if necessary, though localhost/127.0.0.1 is usually safer for internal calls)
    let host = if host == "0.0.0.0" {
        "127.0.0.1".to_string()
    } else {
        host
    };
    let api_url = format!("http://{}:{}/v1/chat/completions", host, port);

    ws::start(
        OneOfFifteenWebSocket::new(
            state.get_ref().clone(),
            broadcaster.get_ref().clone(),
            api_url,
        ),
        &req,
        stream,
    )
}
