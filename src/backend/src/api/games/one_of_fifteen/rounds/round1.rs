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

/// Handle a timeout in Round 1
pub fn handle_timeout(state: &mut GameState, player_id: &str) -> Vec<OutgoingMessage> {
    // Timeout penalties:
    // 1. Deduct life
    // 2. Do NOT increment "misses" (so 2-miss elimination rule doesn't apply)
    deduct_life(state, player_id);

    // Check for elimination (only if ran out of lives)
    check_elimination(state, player_id);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::games::one_of_fifteen::types::Contestant;
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
                score: 0,
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
            round: Round::Round1,
            active_player_id: Some("player1".to_string()),
            current_question: None,
            timer_start: None,
            decision_pending: false,
            past_questions: vec![],
            player_queue: vec!["player1".to_string()],
            active: true,
            buzzer_queue: vec![],
            last_pointer_id: None,
        }
    }

    #[test]
    fn test_handle_timeout_deducts_life_only() {
        let mut state = create_test_state();

        // Timeout 1
        handle_timeout(&mut state, "player1");

        let c = state.contestants.get("player1").unwrap();
        assert_eq!(c.lives, 2);
        assert_eq!(c.round1_misses, 0, "Misses should not increase on timeout");
        assert_eq!(c.eliminated, false);

        // Timeout 2
        handle_timeout(&mut state, "player1");
        let c = state.contestants.get("player1").unwrap();
        assert_eq!(c.lives, 1);
        assert_eq!(c.round1_misses, 0);
        assert_eq!(c.eliminated, false);

        // Timeout 3 -> Elimination
        handle_timeout(&mut state, "player1");
        let c = state.contestants.get("player1").unwrap();
        assert_eq!(c.lives, 0);
        assert_eq!(c.eliminated, true); // Should be eliminated now
    }
}
