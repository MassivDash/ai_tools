use crate::api::games::one_of_ten::rounds::common::*;
use crate::api::games::one_of_ten::types::{GameState, OutgoingMessage, Round};

/// Handle a player buzzing in
pub fn handle_buzz_in(state: &mut GameState, player_id: &str) -> Vec<OutgoingMessage> {
    // Verify player is valid (not eliminated)
    let is_valid = state
        .contestants
        .get(player_id)
        .map(|c| !c.eliminated)
        .unwrap_or(false);

    if !is_valid {
        return vec![OutgoingMessage::Error {
            message: "Invalid player".to_string(),
        }];
    }

    // Set as active player
    state.active_player_id = Some(player_id.to_string());

    vec![create_state_update(state)]
}

/// Handle a correct answer with decision making
pub fn handle_correct_answer_decision(
    state: &mut GameState,
    player_id: &str,
    decision: &str,
    target_id: Option<String>,
) -> Vec<OutgoingMessage> {
    // Reset question state first
    reset_question_state(state);

    match decision {
        "self" => {
            // Double down - player gets DOUBLE POINTS (20) and another turn!
            award_points(state, player_id, 20);
            state.decision_pending = false;
            state.last_pointer_id = None; // Reset last pointer since they took it themselves
                                          // Keep the same active player for next question
        }
        "point" => {
            // Point to another player - normal points (10)
            award_points(state, player_id, 10);

            if let Some(target) = target_id {
                let is_valid = state
                    .contestants
                    .get(&target)
                    .map(|c| !c.eliminated && c.id != player_id)
                    .unwrap_or(false);

                if is_valid {
                    state.active_player_id = Some(target);
                    state.last_pointer_id = Some(player_id.to_string()); // Record who pointed
                    state.decision_pending = false;
                } else {
                    return vec![OutgoingMessage::Error {
                        message: "Invalid target player".to_string(),
                    }];
                }
            } else {
                return vec![OutgoingMessage::Error {
                    message: "Target player ID required for pointing".to_string(),
                }];
            }
        }
        _ => {
            return vec![OutgoingMessage::Error {
                message: "Invalid decision".to_string(),
            }];
        }
    }

    // Check for winner
    if check_winner(state).is_some() {
        end_game(state);
    }

    vec![create_state_update(state)]
}

/// Handle a wrong answer in Round 3 (lose a life, return control to pointer or buzzer)
pub fn handle_wrong_answer(state: &mut GameState, player_id: &str) -> Vec<OutgoingMessage> {
    // Deduct life instead of immediate elimination
    deduct_life(state, player_id);
    let was_eliminated = check_elimination(state, player_id);

    // Reset question state
    reset_question_state(state);
    state.decision_pending = false;

    // If this player was eliminated and they were the last active player, record them as winner
    if was_eliminated && get_active_contestant_ids(state).is_empty() {
        state.winner_id = Some(player_id.to_string());
    }

    // If there is only one active contestant left, they keep control
    let active_ids = get_active_contestant_ids(state);
    if active_ids.len() == 1 {
        state.active_player_id = Some(active_ids[0].clone());
    } else {
        // Check if we should return control to the previous pointer (nominator)
        if let Some(prev_pointer) = state.last_pointer_id.clone() {
            let prev_active = state
                .contestants
                .get(&prev_pointer)
                .map(|c| !c.eliminated)
                .unwrap_or(false);

            if prev_active {
                // Control returns to the nominator
                state.active_player_id = Some(prev_pointer);
            } else {
                // Nominator is eliminated/inactive, control goes back to buzzer
                state.active_player_id = None;
                state.last_pointer_id = None;
            }
        } else {
            // No nominator, control goes back to buzzer
            state.active_player_id = None;
        }
    }

    // Check for winner
    if check_winner(state).is_some() {
        end_game(state);
    }

    vec![create_state_update(state)]
}

/// Check if there's a winner (all players eliminated)
pub fn check_winner(state: &GameState) -> Option<String> {
    if let Some(winner_id) = &state.winner_id {
        return Some(winner_id.clone());
    }

    let active_ids = get_active_contestant_ids(state);

    if active_ids.is_empty() {
        // Find player with the highest score
        state
            .contestants
            .values()
            .max_by_key(|c| c.score)
            .map(|c| c.id.clone())
    } else {
        None
    }
}

/// End the game
pub fn end_game(state: &mut GameState) {
    if state.winner_id.is_none() {
        state.winner_id = check_winner(state);
    }
    state.round = Round::Finished;
    state.active_player_id = None;
    state.decision_pending = false;
    reset_question_state(state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::games::one_of_ten::types::{Contestant, GameState, Round};
    use std::collections::HashMap;

    fn create_test_state() -> GameState {
        let mut contestants = HashMap::new();
        contestants.insert(
            "player1".to_string(),
            Contestant {
                id: "player1".to_string(),
                session_id: "player1".to_string(),
                name: "Player 1".to_string(),
                age: "25".to_string(),
                score: 10,
                online: true,
                ready: true,
                lives: 1,
                round1_misses: 0,
                round1_questions: 0,
                eliminated: false,
            },
        );
        contestants.insert(
            "player2".to_string(),
            Contestant {
                id: "player2".to_string(),
                session_id: "player2".to_string(),
                name: "Player 2".to_string(),
                age: "25".to_string(),
                score: 20,
                online: true,
                ready: true,
                lives: 0,
                round1_misses: 0,
                round1_questions: 0,
                eliminated: true,
            },
        );

        GameState {
            presenter_id: Some("presenter".to_string()),
            presenter_online: true,
            contestants,
            round: Round::Round3,
            active_player_id: Some("player1".to_string()),
            current_question: None,
            timer_start: None,
            decision_pending: false,
            round3_exclusive: false,
            past_questions: vec![],
            player_queue: vec!["player1".to_string(), "player2".to_string()],
            active: true,
            buzzer_queue: vec![],
            last_pointer_id: None,
            last_answer_correct: None,
            last_correct_answer: None,
            waiting_for_presenter: false,
            deferred_action: None,
            winner_id: None,
        }
    }

    fn contestant(id: &str, score: i32, lives: i32) -> Contestant {
        Contestant {
            id: id.to_string(),
            session_id: id.to_string(),
            name: id.to_string(),
            age: "25".to_string(),
            score,
            online: true,
            ready: true,
            lives,
            round1_misses: 0,
            round1_questions: 0,
            eliminated: false,
        }
    }

    /// A Round 3 game with `(id, score, lives)` seated in order.
    fn state_with(players: &[(&str, i32, i32)]) -> GameState {
        let mut state = GameState::new();
        state.presenter_id = Some("presenter".to_string());
        state.presenter_online = true;
        state.round = Round::Round3;
        for (id, score, lives) in players {
            state
                .contestants
                .insert(id.to_string(), contestant(id, *score, *lives));
            state.player_queue.push(id.to_string());
        }
        state
    }

    fn snapshot_of(
        messages: &[OutgoingMessage],
    ) -> &crate::api::games::one_of_ten::types::GameStateSnapshot {
        assert_eq!(messages.len(), 1, "expected exactly one state update");
        match &messages[0] {
            OutgoingMessage::StateUpdate(snapshot) => snapshot,
            other => panic!("Expected a StateUpdate, got {:?}", other),
        }
    }

    fn error_of(messages: &[OutgoingMessage]) -> &str {
        assert_eq!(messages.len(), 1, "expected exactly one error");
        match &messages[0] {
            OutgoingMessage::Error { message } => message,
            other => panic!("Expected an Error, got {:?}", other),
        }
    }

    #[test]
    fn test_buzz_in_takes_control() {
        let mut state = state_with(&[("p1", 0, 3), ("p2", 0, 3)]);

        let messages = handle_buzz_in(&mut state, "p2");

        assert_eq!(state.active_player_id, Some("p2".to_string()));
        assert_eq!(
            snapshot_of(&messages).active_player_id,
            Some("p2".to_string())
        );
    }

    #[test]
    fn test_buzz_in_rejects_unknown_and_eliminated_players() {
        let mut state = state_with(&[("p1", 0, 3), ("p2", 0, 0)]);
        state.contestants.get_mut("p2").unwrap().eliminated = true;

        for id in ["ghost", "p2"] {
            let messages = handle_buzz_in(&mut state, id);
            assert_eq!(error_of(&messages), "Invalid player", "for {}", id);
        }

        assert!(state.active_player_id.is_none());
    }

    #[test]
    fn test_decision_self_doubles_the_points_and_keeps_the_turn() {
        let mut state = state_with(&[("p1", 0, 3), ("p2", 0, 3)]);
        state.active_player_id = Some("p1".to_string());
        state.decision_pending = true;
        state.last_pointer_id = Some("p2".to_string());
        state.timer_start = Some(42);

        let messages = handle_correct_answer_decision(&mut state, "p1", "self", None);

        assert_eq!(state.contestants["p1"].score, 20);
        assert_eq!(state.active_player_id, Some("p1".to_string()));
        assert!(!state.decision_pending);
        assert!(
            state.last_pointer_id.is_none(),
            "taking the question yourself clears the nominator"
        );
        assert!(state.timer_start.is_none());
        assert_eq!(state.round, Round::Round3);
        assert_eq!(snapshot_of(&messages).winner_id, None);
    }

    #[test]
    fn test_decision_point_awards_ten_and_hands_over_control() {
        let mut state = state_with(&[("p1", 0, 3), ("p2", 0, 3)]);
        state.active_player_id = Some("p1".to_string());
        state.decision_pending = true;

        let messages =
            handle_correct_answer_decision(&mut state, "p1", "point", Some("p2".to_string()));

        assert_eq!(state.contestants["p1"].score, 10);
        assert_eq!(state.active_player_id, Some("p2".to_string()));
        assert_eq!(state.last_pointer_id, Some("p1".to_string()));
        assert!(!state.decision_pending);
        assert_eq!(
            snapshot_of(&messages).active_player_id,
            Some("p2".to_string())
        );
    }

    #[test]
    fn test_decision_point_requires_a_valid_target() {
        let mut state = state_with(&[("p1", 0, 3), ("p2", 0, 0), ("p3", 0, 3)]);
        state.contestants.get_mut("p2").unwrap().eliminated = true;
        state.active_player_id = Some("p1".to_string());
        state.decision_pending = true;

        // Missing target
        let messages = handle_correct_answer_decision(&mut state, "p1", "point", None);
        assert_eq!(
            error_of(&messages),
            "Target player ID required for pointing"
        );

        // Eliminated target, unknown target and pointing at yourself
        for target in ["p2", "ghost", "p1"] {
            let messages =
                handle_correct_answer_decision(&mut state, "p1", "point", Some(target.to_string()));
            assert_eq!(
                error_of(&messages),
                "Invalid target player",
                "for {}",
                target
            );
        }

        assert_eq!(
            state.active_player_id,
            Some("p1".to_string()),
            "a rejected decision must not move control"
        );
        assert!(state.decision_pending);
        assert_eq!(
            state.contestants["p1"].score, 40,
            "the 10 points are awarded before the target is validated, once per attempt"
        );
    }

    #[test]
    fn test_unknown_decision_is_rejected_without_awarding_points() {
        let mut state = state_with(&[("p1", 0, 3), ("p2", 0, 3)]);
        state.decision_pending = true;

        let messages = handle_correct_answer_decision(&mut state, "p1", "shrug", None);

        assert_eq!(error_of(&messages), "Invalid decision");
        assert_eq!(state.contestants["p1"].score, 0);
        assert!(state.decision_pending);
    }

    #[test]
    fn test_a_decision_that_leaves_nobody_active_ends_the_game() {
        let mut state = state_with(&[("p1", 30, 0), ("p2", 10, 0)]);
        state.contestants.get_mut("p1").unwrap().eliminated = true;
        state.contestants.get_mut("p2").unwrap().eliminated = true;
        state.active_player_id = Some("p1".to_string());

        let messages = handle_correct_answer_decision(&mut state, "p1", "self", None);

        assert_eq!(state.round, Round::Finished);
        assert_eq!(
            state.winner_id,
            Some("p1".to_string()),
            "the highest score wins once everyone is out"
        );
        assert!(state.active_player_id.is_none());
        assert_eq!(snapshot_of(&messages).winner_id, Some("p1".to_string()));
    }

    #[test]
    fn test_wrong_answer_returns_control_to_the_nominator() {
        let mut state = state_with(&[("p1", 0, 3), ("p2", 0, 3), ("p3", 0, 3)]);
        state.active_player_id = Some("p2".to_string());
        state.last_pointer_id = Some("p1".to_string());
        state.decision_pending = true;

        let messages = handle_wrong_answer(&mut state, "p2");

        assert_eq!(state.contestants["p2"].lives, 2);
        assert!(!state.contestants["p2"].eliminated);
        assert_eq!(state.active_player_id, Some("p1".to_string()));
        assert!(!state.decision_pending);
        assert_eq!(state.round, Round::Round3);
        assert_eq!(
            snapshot_of(&messages).active_player_id,
            Some("p1".to_string())
        );
    }

    #[test]
    fn test_wrong_answer_returns_to_the_buzzer_when_the_nominator_is_out() {
        let mut state = state_with(&[("p1", 0, 0), ("p2", 0, 3), ("p3", 0, 3)]);
        state.contestants.get_mut("p1").unwrap().eliminated = true;
        state.active_player_id = Some("p2".to_string());
        state.last_pointer_id = Some("p1".to_string());

        handle_wrong_answer(&mut state, "p2");

        assert!(state.active_player_id.is_none(), "back to the buzzer");
        assert!(state.last_pointer_id.is_none());
    }

    #[test]
    fn test_wrong_answer_without_a_nominator_returns_to_the_buzzer() {
        let mut state = state_with(&[("p1", 0, 3), ("p2", 0, 3), ("p3", 0, 3)]);
        state.active_player_id = Some("p2".to_string());

        handle_wrong_answer(&mut state, "p2");

        assert!(state.active_player_id.is_none());
        assert_eq!(state.round, Round::Round3);
    }

    #[test]
    fn test_check_winner_prefers_a_recorded_winner_over_the_score() {
        let mut state = state_with(&[("p1", 10, 3), ("p2", 99, 3)]);
        assert_eq!(
            check_winner(&state),
            None,
            "nobody wins while players are still in"
        );

        state.winner_id = Some("p1".to_string());
        assert_eq!(check_winner(&state), Some("p1".to_string()));
    }

    #[test]
    fn test_check_winner_falls_back_to_the_highest_score() {
        let mut state = state_with(&[("p1", 10, 0), ("p2", 99, 0)]);
        for id in ["p1", "p2"] {
            state.contestants.get_mut(id).unwrap().eliminated = true;
        }

        assert_eq!(check_winner(&state), Some("p2".to_string()));

        // With no contestants at all there is nobody to crown
        assert_eq!(check_winner(&GameState::new()), None);
    }

    #[test]
    fn test_end_game_finishes_and_records_the_winner() {
        let mut state = state_with(&[("p1", 10, 0), ("p2", 99, 0)]);
        for id in ["p1", "p2"] {
            state.contestants.get_mut(id).unwrap().eliminated = true;
        }
        state.active_player_id = Some("p1".to_string());
        state.decision_pending = true;
        state.timer_start = Some(42);
        state.current_question = Some(crate::api::games::one_of_ten::types::Question {
            text: "2 + 2?".to_string(),
            correct_answer: "4".to_string(),
            options: None,
        });

        end_game(&mut state);

        assert_eq!(state.round, Round::Finished);
        assert_eq!(state.winner_id, Some("p2".to_string()));
        assert!(state.active_player_id.is_none());
        assert!(!state.decision_pending);
        assert!(state.timer_start.is_none());
        assert!(state.current_question.is_none());
    }

    #[test]
    fn test_end_game_keeps_an_already_recorded_winner() {
        let mut state = state_with(&[("p1", 10, 3), ("p2", 99, 3)]);
        state.winner_id = Some("p1".to_string());

        end_game(&mut state);

        assert_eq!(state.winner_id, Some("p1".to_string()));
        assert_eq!(state.round, Round::Finished);
    }

    #[test]
    fn test_last_player_elimination_sets_winner() {
        let mut state = create_test_state();

        assert_eq!(state.winner_id, None);
        assert_eq!(check_winner(&state), None);

        let _msgs = handle_wrong_answer(&mut state, "player1");

        assert_eq!(state.winner_id, Some("player1".to_string()));
        assert_eq!(state.round, Round::Finished);
        assert_eq!(check_winner(&state), Some("player1".to_string()));
    }

    #[test]
    fn test_single_active_player_keeps_control() {
        let mut state = create_test_state();

        // Give player1 more lives so they survive a wrong answer
        state.contestants.get_mut("player1").unwrap().lives = 3;

        let _msgs = handle_wrong_answer(&mut state, "player1");

        assert_eq!(state.active_player_id, Some("player1".to_string()));
        assert_eq!(state.decision_pending, false);
    }
}
