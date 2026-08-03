use crate::api::games::one_of_ten::rounds::common::*;
use crate::api::games::one_of_ten::types::{GameState, OutgoingMessage, Round};

/// Handle pointing to another player in Round 2
pub fn handle_point_to_player(state: &mut GameState, target_id: &str) -> Vec<OutgoingMessage> {
    // Verify target is valid (not eliminated, online)
    let is_valid = state
        .contestants
        .get(target_id)
        .map(|c| !c.eliminated)
        .unwrap_or(false);

    if !is_valid {
        return vec![OutgoingMessage::Error {
            message: "Invalid target player".to_string(),
        }];
    }

    // Set the targeted player as active
    state.active_player_id = Some(target_id.to_string());
    state.decision_pending = false;
    reset_question_state(state);

    vec![create_state_update(state)]
}

/// Handle a correct answer in Round 2
pub fn handle_correct_answer(state: &mut GameState, player_id: &str) -> Vec<OutgoingMessage> {
    // Award points - DISABLED for Round 2

    // Reset question state
    reset_question_state(state);

    // Store the player who answered correctly for potential rollback
    state.last_pointer_id = Some(player_id.to_string());

    // The player who answered correctly stays active to point to the next player
    state.active_player_id = Some(player_id.to_string());

    // Check if we should transition to Round 3
    if check_survivors(state) <= 3 {
        transition_to_round3(state);
    } else {
        state.decision_pending = true;
    }

    vec![create_state_update(state)]
}

pub fn select_next_rotation_player(state: &GameState, current_id: &str) -> Option<String> {
    // Walk the seat order (Player 1..10), skipping eliminated players, so the
    // "move to the next player in numerical order" rule is followed correctly.
    let active_ids: Vec<String> = state
        .player_queue
        .iter()
        .filter(|id| {
            state
                .contestants
                .get(*id)
                .map(|c| !c.eliminated)
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    if active_ids.is_empty() {
        return None;
    }

    if let Some(pos) = active_ids.iter().position(|id| id == current_id) {
        let next_pos = (pos + 1) % active_ids.len();
        Some(active_ids[next_pos].clone())
    } else {
        active_ids.first().cloned()
    }
}

/// Handle a wrong answer in Round 2
pub fn handle_wrong_answer(
    state: &mut GameState,
    player_id: &str,
) -> (
    Vec<OutgoingMessage>,
    Option<crate::api::games::one_of_ten::types::AsyncAction>,
) {
    // Deduct life
    deduct_life(state, player_id);
    check_elimination(state, player_id);

    // Reset question state
    reset_question_state(state);

    let mut action = None;

    // The previous pointer gets to point again
    if let Some(prev_pointer) = state.last_pointer_id.clone() {
        // Check if previous pointer is still active
        let prev_active = state
            .contestants
            .get(&prev_pointer)
            .map(|c| !c.eliminated)
            .unwrap_or(false);

        if prev_active {
            // Return control to the previous pointer
            state.active_player_id = Some(prev_pointer);
        } else {
            // Previous pointer is gone, go to the next player in rotation!
            if let Some(next_id) = select_next_rotation_player(state, player_id) {
                state.active_player_id = Some(next_id.clone());
                if let Some(contestant) = state.contestants.get(&next_id) {
                    action = Some(
                        crate::api::games::one_of_ten::types::AsyncAction::GenerateQuestion {
                            age: contestant.age.clone(),
                            past_questions: state.past_questions.clone(),
                        },
                    );
                }
            }
        }
    } else {
        // No previous pointer (initial rotation), go to the next player in rotation!
        if let Some(next_id) = select_next_rotation_player(state, player_id) {
            state.active_player_id = Some(next_id.clone());
            if let Some(contestant) = state.contestants.get(&next_id) {
                action = Some(
                    crate::api::games::one_of_ten::types::AsyncAction::GenerateQuestion {
                        age: contestant.age.clone(),
                        past_questions: state.past_questions.clone(),
                    },
                );
            }
        }
    }

    // Check if we should transition to Round 3
    if check_survivors(state) <= 3 {
        transition_to_round3(state);
        action = None;
    }

    (vec![create_state_update(state)], action)
}

/// Count number of survivors (non-eliminated players)
pub fn check_survivors(state: &GameState) -> usize {
    count_active_contestants(state)
}

/// Transition from Round 2 to Round 3
pub fn transition_to_round3(state: &mut GameState) {
    state.round = Round::Round3;
    state.active_player_id = None;
    state.decision_pending = false;
    state.last_pointer_id = None;

    // Reset lives for all survivors
    for contestant in state.contestants.values_mut() {
        if !contestant.eliminated {
            contestant.lives = 3;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::games::one_of_ten::types::{AsyncAction, Contestant, GameState, Round};
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
                lives: 3,
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
                score: 10,
                online: true,
                ready: true,
                lives: 3,
                round1_misses: 0,
                round1_questions: 0,
                eliminated: false,
            },
        );
        contestants.insert(
            "player3".to_string(),
            Contestant {
                id: "player3".to_string(),
                session_id: "player3".to_string(),
                name: "Player 3".to_string(),
                age: "25".to_string(),
                score: 10,
                online: true,
                ready: true,
                lives: 3,
                round1_misses: 0,
                round1_questions: 0,
                eliminated: false,
            },
        );
        contestants.insert(
            "player4".to_string(),
            Contestant {
                id: "player4".to_string(),
                session_id: "player4".to_string(),
                name: "Player 4".to_string(),
                age: "25".to_string(),
                score: 10,
                online: true,
                ready: true,
                lives: 3,
                round1_misses: 0,
                round1_questions: 0,
                eliminated: false,
            },
        );

        GameState {
            presenter_id: Some("presenter".to_string()),
            presenter_online: true,
            contestants,
            round: Round::Round2,
            active_player_id: Some("player1".to_string()),
            current_question: None,
            timer_start: None,
            decision_pending: false,
            round3_exclusive: false,
            past_questions: vec![],
            player_queue: vec![
                "player1".to_string(),
                "player2".to_string(),
                "player3".to_string(),
                "player4".to_string(),
            ],
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

    fn snapshot_of(
        messages: &[OutgoingMessage],
    ) -> &crate::api::games::one_of_ten::types::GameStateSnapshot {
        assert_eq!(messages.len(), 1, "expected exactly one state update");
        match &messages[0] {
            OutgoingMessage::StateUpdate(snapshot) => snapshot,
            other => panic!("Expected a StateUpdate, got {:?}", other),
        }
    }

    fn add_player(state: &mut GameState, id: &str) {
        state.contestants.insert(
            id.to_string(),
            Contestant {
                id: id.to_string(),
                session_id: id.to_string(),
                name: id.to_string(),
                age: "25".to_string(),
                score: 10,
                online: true,
                ready: true,
                lives: 3,
                round1_misses: 0,
                round1_questions: 0,
                eliminated: false,
            },
        );
        state.player_queue.push(id.to_string());
    }

    fn eliminate(state: &mut GameState, ids: &[&str]) {
        for id in ids {
            let c = state.contestants.get_mut(*id).expect("unknown contestant");
            c.eliminated = true;
            c.lives = 0;
        }
    }

    #[test]
    fn test_point_to_player_rejects_an_unknown_or_eliminated_target() {
        let mut state = create_test_state();
        state.decision_pending = true;
        eliminate(&mut state, &["player3"]);

        for target in ["ghost", "player3"] {
            let messages = handle_point_to_player(&mut state, target);
            assert_eq!(messages.len(), 1);
            match &messages[0] {
                OutgoingMessage::Error { message } => assert_eq!(message, "Invalid target player"),
                other => panic!("Expected an error for {}, got {:?}", target, other),
            }
        }

        assert_eq!(
            state.active_player_id,
            Some("player1".to_string()),
            "a rejected pointing must not move control"
        );
        assert!(state.decision_pending, "the decision is still pending");
    }

    #[test]
    fn test_point_to_player_clears_the_pending_decision_and_question() {
        let mut state = create_test_state();
        state.decision_pending = true;
        state.current_question = Some(crate::api::games::one_of_ten::types::Question {
            text: "2 + 2?".to_string(),
            correct_answer: "4".to_string(),
            options: None,
        });
        state.timer_start = Some(42);

        let messages = handle_point_to_player(&mut state, "player2");

        assert_eq!(state.active_player_id, Some("player2".to_string()));
        assert!(!state.decision_pending);
        assert!(state.current_question.is_none());
        assert!(state.timer_start.is_none());
        assert_eq!(
            snapshot_of(&messages).active_player_id,
            Some("player2".to_string())
        );
    }

    #[test]
    fn test_correct_answer_keeps_control_and_records_the_pointer() {
        let mut state = create_test_state();
        state.timer_start = Some(42);

        let messages = handle_correct_answer(&mut state, "player2");

        assert_eq!(state.active_player_id, Some("player2".to_string()));
        assert_eq!(state.last_pointer_id, Some("player2".to_string()));
        assert!(state.decision_pending);
        assert!(state.timer_start.is_none());
        assert_eq!(
            state.contestants["player2"].score, 10,
            "scoring is disabled in round 2, the seed score is untouched"
        );
        assert!(snapshot_of(&messages).decision_pending);
    }

    #[test]
    fn test_correct_answer_transitions_to_round3_once_three_survive() {
        let mut state = create_test_state();
        eliminate(&mut state, &["player4"]);
        // Give the survivors battle damage so the round 3 reset is observable
        for id in ["player1", "player2", "player3"] {
            state.contestants.get_mut(id).unwrap().lives = 1;
        }

        let messages = handle_correct_answer(&mut state, "player1");

        assert_eq!(state.round, Round::Round3);
        assert!(state.active_player_id.is_none());
        assert!(!state.decision_pending);
        assert!(state.last_pointer_id.is_none());
        for id in ["player1", "player2", "player3"] {
            assert_eq!(
                state.contestants[id].lives, 3,
                "{} should have their lives restored",
                id
            );
        }
        assert_eq!(
            state.contestants["player4"].lives, 0,
            "eliminated players are not revived"
        );
        assert_eq!(snapshot_of(&messages).round, Round::Round3);
    }

    #[test]
    fn test_wrong_answer_returns_control_to_the_previous_pointer() {
        let mut state = create_test_state();
        state.last_pointer_id = Some("player1".to_string());
        state.active_player_id = Some("player2".to_string());

        let (messages, action) = handle_wrong_answer(&mut state, "player2");

        assert_eq!(state.contestants["player2"].lives, 2);
        assert!(!state.contestants["player2"].eliminated);
        assert_eq!(state.active_player_id, Some("player1".to_string()));
        assert!(
            action.is_none(),
            "the pointer asks the next question, so no question is generated"
        );
        assert_eq!(
            snapshot_of(&messages).active_player_id,
            Some("player1".to_string())
        );
    }

    #[test]
    fn test_wrong_answer_without_a_pointer_rotates_and_asks_for_a_question() {
        let mut state = create_test_state();
        state.active_player_id = Some("player1".to_string());

        let (_messages, action) = handle_wrong_answer(&mut state, "player1");

        assert_eq!(state.active_player_id, Some("player2".to_string()));
        match action.expect("expected a GenerateQuestion action") {
            AsyncAction::GenerateQuestion {
                age,
                past_questions,
            } => {
                assert_eq!(age, "25", "the question is tailored to the next player");
                assert!(past_questions.is_empty());
            }
            other => panic!("Expected GenerateQuestion, got {:?}", other),
        }
    }

    #[test]
    fn test_wrong_answer_rotates_when_the_previous_pointer_is_eliminated() {
        let mut state = create_test_state();
        // A fifth seat keeps the survivor count above the round 3 threshold
        add_player(&mut state, "player5");
        state.last_pointer_id = Some("player4".to_string());
        eliminate(&mut state, &["player4"]);
        state.active_player_id = Some("player1".to_string());
        state.past_questions = vec!["an old question".to_string()];

        let (_messages, action) = handle_wrong_answer(&mut state, "player1");

        assert_eq!(
            state.active_player_id,
            Some("player2".to_string()),
            "control moves to the next surviving seat, not the dead pointer"
        );
        match action.expect("expected a GenerateQuestion action") {
            AsyncAction::GenerateQuestion { past_questions, .. } => {
                assert_eq!(past_questions, vec!["an old question"]);
            }
            other => panic!("Expected GenerateQuestion, got {:?}", other),
        }
    }

    #[test]
    fn test_wrong_answer_that_eliminates_a_player_can_trigger_round3() {
        let mut state = create_test_state();
        state.contestants.get_mut("player4").unwrap().lives = 1;
        state.last_pointer_id = Some("player1".to_string());

        let (messages, action) = handle_wrong_answer(&mut state, "player4");

        assert!(state.contestants["player4"].eliminated);
        assert_eq!(state.round, Round::Round3);
        assert!(
            action.is_none(),
            "a queued question is dropped when the round changes"
        );
        assert!(state.active_player_id.is_none());
        assert_eq!(snapshot_of(&messages).round, Round::Round3);
    }

    #[test]
    fn test_select_next_rotation_player_walks_seat_order() {
        let mut state = create_test_state();

        assert_eq!(
            select_next_rotation_player(&state, "player1"),
            Some("player2".to_string())
        );
        assert_eq!(
            select_next_rotation_player(&state, "player4"),
            Some("player1".to_string()),
            "the rotation wraps around"
        );
        assert_eq!(
            select_next_rotation_player(&state, "ghost"),
            Some("player1".to_string()),
            "an unknown current player restarts at the first seat"
        );

        eliminate(&mut state, &["player2", "player3"]);
        assert_eq!(
            select_next_rotation_player(&state, "player1"),
            Some("player4".to_string()),
            "eliminated seats are skipped"
        );

        eliminate(&mut state, &["player1", "player4"]);
        assert_eq!(select_next_rotation_player(&state, "player1"), None);
    }

    #[test]
    fn test_check_survivors_counts_non_eliminated_players() {
        let mut state = create_test_state();
        assert_eq!(check_survivors(&state), 4);

        eliminate(&mut state, &["player1", "player2"]);
        assert_eq!(check_survivors(&state), 2);

        assert_eq!(check_survivors(&GameState::new()), 0);
    }

    #[test]
    fn test_round2_pointing_flow() {
        let mut state = create_test_state();

        assert_eq!(state.decision_pending, false);

        let _msgs = handle_correct_answer(&mut state, "player1");
        assert_eq!(state.decision_pending, true);
        assert_eq!(state.active_player_id, Some("player1".to_string()));

        let _msgs = handle_point_to_player(&mut state, "player2");
        assert_eq!(state.decision_pending, false);
        assert_eq!(state.active_player_id, Some("player2".to_string()));

        let (_msgs, _action) = handle_wrong_answer(&mut state, "player2");
        assert_eq!(state.decision_pending, false);
    }
}
