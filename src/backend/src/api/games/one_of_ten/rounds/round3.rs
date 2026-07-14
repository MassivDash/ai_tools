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
}
