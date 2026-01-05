use crate::api::games::one_of_fifteen::player_selection;
use crate::api::games::one_of_fifteen::rounds::common::*;
use crate::api::games::one_of_fifteen::types::{GameState, OutgoingMessage, Round};

/// Handle a correct answer in Round 1
pub fn handle_correct_answer(state: &mut GameState, player_id: &str) -> Vec<OutgoingMessage> {
    // Award points
    award_points(state, player_id, 10);

    // Increment question count
    if let Some(contestant) = state.contestants.get_mut(player_id) {
        contestant.round1_questions += 1;
    }

    // Reset question state
    reset_question_state(state);

    // Check if round is complete
    if check_round1_complete(state) {
        transition_to_round2(state);
    } else {
        // Move to next player
        if let Some(next_id) =
            player_selection::select_next_player(state, Some(player_id), &Round::Round1)
        {
            state.active_player_id = Some(next_id);
        }
    }

    vec![create_state_update(state)]
}

/// Handle a wrong answer in Round 1
pub fn handle_wrong_answer(state: &mut GameState, player_id: &str) -> Vec<OutgoingMessage> {
    // Track miss and deduct life
    if let Some(contestant) = state.contestants.get_mut(player_id) {
        contestant.round1_misses += 1;
        contestant.round1_questions += 1;
    }

    deduct_life(state, player_id);

    // Check for elimination (2 misses in Round 1)
    if let Some(contestant) = state.contestants.get(player_id) {
        if contestant.round1_misses >= 2 {
            if let Some(c) = state.contestants.get_mut(player_id) {
                c.eliminated = true;
            }
        }
    }

    // Reset question state
    reset_question_state(state);

    // Check if round is complete
    if check_round1_complete(state) {
        transition_to_round2(state);
    } else {
        // Move to next player
        if let Some(next_id) =
            player_selection::select_next_player(state, Some(player_id), &Round::Round1)
        {
            state.active_player_id = Some(next_id);
        }
    }

    vec![create_state_update(state)]
}

/// Check if Round 1 is complete (all active players answered 2 questions)
pub fn check_round1_complete(state: &GameState) -> bool {
    let active = get_active_contestant_ids(state);
    if active.is_empty() {
        return true;
    }

    active.iter().all(|id| {
        state
            .contestants
            .get(id)
            .map(|c| c.round1_questions >= 2)
            .unwrap_or(false)
    })
}

/// Transition from Round 1 to Round 2
pub fn transition_to_round2(state: &mut GameState) {
    state.round = Round::Round2;
    state.active_player_id = None;
    state.decision_pending = false;

    // Pick a random first player for Round 2
    if let Some(first_id) = player_selection::select_random_active(state) {
        state.active_player_id = Some(first_id);
    }
}
