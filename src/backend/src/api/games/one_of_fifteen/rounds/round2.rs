use crate::api::games::one_of_fifteen::rounds::common::*;
use crate::api::games::one_of_fifteen::types::{GameState, OutgoingMessage, Round};

/// Handle pointing to another player in Round 2
pub fn handle_point_to_player(state: &mut GameState, target_id: &str) -> Vec<OutgoingMessage> {
    // Verify target is valid (not eliminated, online)
    let is_valid = state
        .contestants
        .get(target_id)
        .map(|c| !c.eliminated && c.online)
        .unwrap_or(false);

    if !is_valid {
        return vec![OutgoingMessage::Error {
            message: "Invalid target player".to_string(),
        }];
    }

    // Set the targeted player as active
    state.active_player_id = Some(target_id.to_string());
    reset_question_state(state);

    vec![create_state_update(state)]
}

/// Handle a correct answer in Round 2
pub fn handle_correct_answer(state: &mut GameState, player_id: &str) -> Vec<OutgoingMessage> {
    // Award points
    award_points(state, player_id, 10);

    // Reset question state
    reset_question_state(state);

    // Store the player who answered correctly for potential rollback
    state.last_pointer_id = Some(player_id.to_string());

    // The player who answered correctly stays active to point to the next player
    state.active_player_id = Some(player_id.to_string());

    // Check if we should transition to Round 3
    if check_survivors(state) <= 3 {
        transition_to_round3(state);
    }

    vec![create_state_update(state)]
}

/// Handle a wrong answer in Round 2
pub fn handle_wrong_answer(state: &mut GameState, player_id: &str) -> Vec<OutgoingMessage> {
    // Deduct life
    deduct_life(state, player_id);
    check_elimination(state, player_id);

    // Reset question state
    reset_question_state(state);

    // The previous pointer gets to point again
    if let Some(prev_pointer) = state.last_pointer_id.clone() {
        // Check if previous pointer is still active
        let prev_active = state
            .contestants
            .get(&prev_pointer)
            .map(|c| !c.eliminated && c.online)
            .unwrap_or(false);

        if prev_active {
            // Return control to the previous pointer
            state.active_player_id = Some(prev_pointer);
        } else {
            // Previous pointer is gone, pick random
            state.active_player_id = None;
            if let Some(random_id) =
                crate::api::games::one_of_fifteen::player_selection::select_random_active(state)
            {
                state.active_player_id = Some(random_id.clone());
                state.last_pointer_id = Some(random_id);
            }
        }
    } else {
        // No previous pointer, pick random
        state.active_player_id = None;
        if let Some(random_id) =
            crate::api::games::one_of_fifteen::player_selection::select_random_active(state)
        {
            state.active_player_id = Some(random_id.clone());
            state.last_pointer_id = Some(random_id);
        }
    }

    // Check if we should transition to Round 3
    if check_survivors(state) <= 3 {
        transition_to_round3(state);
    }

    vec![create_state_update(state)]
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
