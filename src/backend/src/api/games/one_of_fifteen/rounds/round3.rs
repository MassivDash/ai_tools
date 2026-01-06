use crate::api::games::one_of_fifteen::rounds::common::*;
use crate::api::games::one_of_fifteen::types::{GameState, OutgoingMessage, Round};

/// Handle a player buzzing in
pub fn handle_buzz_in(state: &mut GameState, player_id: &str) -> Vec<OutgoingMessage> {
    // Verify player is valid
    let is_valid = state
        .contestants
        .get(player_id)
        .map(|c| !c.eliminated && c.online)
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
            // Double down - player gets DOUBLE POINTS and another turn!
            award_points(state, player_id, 20); // DOUBLED from 10 to 20
            state.decision_pending = false;
            // Keep the same active player for next question
        }
        "point" => {
            // Point to another player - normal points
            award_points(state, player_id, 10);

            if let Some(target) = target_id {
                let is_valid = state
                    .contestants
                    .get(&target)
                    .map(|c| !c.eliminated && c.online && c.id != player_id)
                    .unwrap_or(false);

                if is_valid {
                    state.active_player_id = Some(target);
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
    // Note: Do NOT reset active_player_id here.
    // It has been set to the target player (Self or Pointed) in the match block above.
    // This allows the next question to be targeted specifically to them.

    vec![create_state_update(state)]
}

/// Handle a wrong answer in Round 3 (elimination)
pub fn handle_wrong_answer(state: &mut GameState, player_id: &str) -> Vec<OutgoingMessage> {
    // Immediate elimination in Round 3
    if let Some(contestant) = state.contestants.get_mut(player_id) {
        contestant.eliminated = true;
        contestant.lives = 0;
    }

    // Reset question state
    reset_question_state(state);
    state.active_player_id = None;
    state.decision_pending = false;

    // Check for winner
    if check_winner(state).is_some() {
        end_game(state);
    }

    vec![create_state_update(state)]
}

/// Check if there's a winner (only one player left - fallback)
pub fn check_winner(state: &GameState) -> Option<String> {
    // Round 3 ends ONLY when there is 1 survivor left (Last Man Standing)
    // The 30-point threshold is only for enabling the "Decision" mechanic.

    let active_ids = get_active_contestant_ids(state);

    if active_ids.len() == 1 {
        active_ids.first().cloned()
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
