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

pub struct OneOfTenWebSocket {
    hb: Instant,
    state: GameStateHandle,
    id: String,
    role: UserRole,
    broadcaster: super::BroadcastHandle,
    ai_api_url: String,
}

impl OneOfTenWebSocket {
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
                if state.round != Round::Lobby {
                    responses.push(OutgoingMessage::Error {
                        message: "Game is already in progress".to_string(),
                    });
                    return (responses, None);
                }

                if state.contestants.len() >= 10 && !state.contestants.contains_key(connection_id) {
                    responses.push(OutgoingMessage::Error {
                        message: "Game is full (maximum 10 players)".to_string(),
                    });
                    return (responses, None);
                }

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
                if !state.player_queue.contains(&session_id) {
                    state.player_queue.push(session_id.clone());
                }
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
                    // player_queue already holds players in the order they joined
                    // (Player 1 .. Player 10) - that's the seat order the host follows.

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
                    // Keep player_queue as-is: contestants stay connected across a reset,
                    // so their join order (seat order) should carry over to the next game.
                    state.active = true; // Ensure game is active
                    state.round3_exclusive = false; // Reset Exclusive Mode
                    state.winner_id = None;
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
                    if state.round == Round::Finished {
                        action = None;
                    } else if let Some(target_player_id) = &state.active_player_id {
                        if let Some(player) = state.contestants.get(target_player_id) {
                            action = Some(AsyncAction::GenerateQuestion {
                                age: player.age.clone(),
                                past_questions: state.past_questions.clone(),
                            });
                        }
                    } else {
                        // If for some reason no active player, fallback to random (shouldn't happen in logic)
                        if let Some(random_id) =
                            crate::api::games::one_of_ten::player_selection::select_random_active(
                                state,
                            )
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
            IncomingMessage::PresenterFinishedSpeaking => {
                if state.presenter_id.as_ref() == Some(connection_id) {
                    state.waiting_for_presenter = false;
                    action = state.deferred_action.take();
                    responses.push(rounds::common::create_state_update(state));
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
                move |res, _, _ctx: &mut ws::WebsocketContext<OneOfTenWebSocket>| {
                    if let Some(q) = res {
                        let mut state = state.lock().unwrap();
                        state.past_questions.push(q.text.clone());
                        state.current_question = Some(q);
                        state.last_answer_correct = None;
                        state.last_correct_answer = None;

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
        state.last_answer_correct = Some(is_correct);
        if !is_correct {
            state.last_correct_answer = state
                .current_question
                .as_ref()
                .map(|q| q.correct_answer.clone());
        } else {
            state.last_correct_answer = None;
        }

        let round = state.round.clone();

        // Delegate to round-specific logic
        let (msgs, next_action) = match round {
            Round::Round1 => {
                let msgs = if is_correct {
                    rounds::round1::handle_correct_answer(state, &player_id)
                } else {
                    rounds::round1::handle_wrong_answer(state, &player_id)
                };

                // Check if there's a next player who needs a question
                let action = if let Some(next_player_id) = &state.active_player_id {
                    // Only generate if we are still in Round 1.
                    // If we transitioned to Round 2, the active player (first in alphabetical order)
                    // should be generated a question below.
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

                // Check if we transitioned to Round 2
                let final_action = if state.round == Round::Round2 {
                    if let Some(next_id) = &state.active_player_id {
                        if let Some(contestant) = state.contestants.get(next_id) {
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
                    action
                };

                (msgs, final_action)
            }
            Round::Round2 => {
                let (msgs, action) = if is_correct {
                    (
                        rounds::round2::handle_correct_answer(state, &player_id),
                        None,
                    )
                } else {
                    rounds::round2::handle_wrong_answer(state, &player_id)
                };

                // Check if we transitioned to Round 3
                let final_action = if state.round == Round::Round3 {
                    // Generate first question for Round 3
                    if let Some(random_id) =
                        crate::api::games::one_of_ten::player_selection::select_random_active(state)
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
                    action
                };

                (msgs, final_action)
            }
            Round::Round3 => {
                if is_correct {
                    // Check if they are the only player left
                    if rounds::common::count_active_contestants(state) == 1 {
                        rounds::common::award_points(state, &player_id, 10);
                        state.round3_exclusive = true;
                        state.decision_pending = false;
                        rounds::common::reset_question_state(state);
                        
                        let action = Some(AsyncAction::GenerateQuestion {
                            age: state.contestants.get(&player_id).unwrap().age.clone(),
                            past_questions: state.past_questions.clone(),
                        });
                        (vec![rounds::common::create_state_update(state)], action)
                    } else {
                        // In Jeden z dziesięciu, any correct answer in Round 3 gives the player control!
                        state.round3_exclusive = true;
                        state.decision_pending = true;
                        rounds::common::reset_question_state(state);

                        // STOP question generation logic here.
                        // We must wait for the player to make a Decision (Point/Self).
                        // Do NOT clear active_player_id. It stays with the correct answerer.
                        (vec![rounds::common::create_state_update(state)], None)
                    }
                } else {
                    let msgs = rounds::round3::handle_wrong_answer(state, &player_id);

                    // Generate next question
                    // If active_player_id is Some(next_id), then that player has control (retained or returned).
                    // If active_player_id is None, it is a buzzer question.
                    let action = if state.round == Round::Finished {
                        None
                    } else if rounds::common::count_active_contestants(state) == 1 {
                        state.round3_exclusive = true;
                        state.decision_pending = false;
                        if let Some(active_id) = &state.active_player_id {
                            if let Some(player) = state.contestants.get(active_id) {
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
                    } else if let Some(_next_id) = &state.active_player_id {
                        state.round3_exclusive = true;
                        state.decision_pending = true;
                        None
                    } else {
                        state.round3_exclusive = false;
                        state.decision_pending = false;
                        if let Some(random_id) =
                            crate::api::games::one_of_ten::player_selection::select_random_active(
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
                    };

                    (msgs, action)
                }
            }
            _ => (vec![rounds::common::create_state_update(state)], None),
        };

        if state.presenter_online && next_action.is_some() {
            state.waiting_for_presenter = true;
            state.deferred_action = next_action;
            let mut final_msgs = msgs;
            final_msgs.push(rounds::common::create_state_update(state));
            (final_msgs, None)
        } else {
            (msgs, next_action)
        }
    }

    fn handle_timeout(
        state: &mut GameState,
        player_id: String,
    ) -> (Vec<OutgoingMessage>, Option<AsyncAction>) {
        state.last_answer_correct = Some(false);
        state.last_correct_answer = state
            .current_question
            .as_ref()
            .map(|q| q.correct_answer.clone());

        let round = state.round.clone();

        let (msgs, next_action) = match round {
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

                // Check if we transitioned to Round 2
                let final_action = if state.round == Round::Round2 {
                    if let Some(next_id) = &state.active_player_id {
                        if let Some(contestant) = state.contestants.get(next_id) {
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
                    action
                };

                (msgs, final_action)
            }
            // For other rounds, fall back to logic treating timeout as wrong answer if not specified
            Round::Round2 => {
                let (msgs, action) = rounds::round2::handle_wrong_answer(state, &player_id);

                // Check if we transitioned to Round 3
                let final_action = if state.round == Round::Round3 {
                    if let Some(random_id) =
                        crate::api::games::one_of_ten::player_selection::select_random_active(state)
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
                    action
                };

                (msgs, final_action)
            }
            Round::Round3 => {
                let msgs = rounds::round3::handle_wrong_answer(state, &player_id);
                let action = if state.round == Round::Finished {
                    None
                } else if rounds::common::count_active_contestants(state) == 1 {
                    state.round3_exclusive = true;
                    state.decision_pending = false;
                    if let Some(active_id) = &state.active_player_id {
                        if let Some(player) = state.contestants.get(active_id) {
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
                } else if let Some(_next_id) = &state.active_player_id {
                    state.round3_exclusive = true;
                    state.decision_pending = true;
                    None
                } else {
                    state.round3_exclusive = false;
                    state.decision_pending = false;
                    if let Some(random_id) =
                        crate::api::games::one_of_ten::player_selection::select_random_active(state)
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
                };
                (msgs, action)
            }
            _ => (vec![rounds::common::create_state_update(state)], None),
        };

        if state.presenter_online && next_action.is_some() {
            state.waiting_for_presenter = true;
            state.deferred_action = next_action;
            let mut final_msgs = msgs;
            final_msgs.push(rounds::common::create_state_update(state));
            (final_msgs, None)
        } else {
            (msgs, next_action)
        }
    }
}

impl Actor for OneOfTenWebSocket {
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
                    update_needed = true;
                }
            }
            UserRole::Contestant => {
                if let Some(contestant) = state.contestants.get_mut(&self.id) {
                    contestant.online = false;
                    update_needed = true;
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

impl Handler<BroadcastingMessage> for OneOfTenWebSocket {
    type Result = ();

    fn handle(&mut self, msg: BroadcastingMessage, ctx: &mut Self::Context) {
        if let Ok(json) = serde_json::to_string(&msg.0) {
            ctx.text(json);
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for OneOfTenWebSocket {
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
                                            OneOfTenWebSocket,
                                        >| {
                                            let state_clone = state.clone();
                                            let mut state_lock = state.lock().unwrap();

                                            let (msgs, next_action) =
                                                OneOfTenWebSocket::handle_validate_answer(
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

#[get("/api/games/1-z-10/ws")]
pub async fn one_of_ten_ws_route(
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
        OneOfTenWebSocket::new(
            state.get_ref().clone(),
            broadcaster.get_ref().clone(),
            api_url,
        ),
        &req,
        stream,
    )
}
