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
    check_elimination(state, player_id);

    // Reset question state
    reset_question_state(state);
    state.decision_pending = false;

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
    state.round = Round::Finished;
    state.active_player_id = None;
    state.decision_pending = false;
    reset_question_state(state);
}
