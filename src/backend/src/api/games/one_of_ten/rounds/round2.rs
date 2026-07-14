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
