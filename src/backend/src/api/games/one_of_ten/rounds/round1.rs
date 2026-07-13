use crate::api::games::one_of_ten::player_selection;
use crate::api::games::one_of_ten::rounds::common::*;
use crate::api::games::one_of_ten::types::{GameState, OutgoingMessage, Round};

/// Handle a correct answer in Round 1
pub fn handle_correct_answer(state: &mut GameState, player_id: &str) -> Vec<OutgoingMessage> {
    // Award points - DISABLED for Round 1

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
    // Track miss and deduct life. No eliminations happen in Round 1 - a player who
    // misses both questions simply carries only 1 life into Round 2.
    if let Some(contestant) = state.contestants.get_mut(player_id) {
        contestant.round1_misses += 1;
        contestant.round1_questions += 1;
    }

    deduct_life(state, player_id);

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
    // Track miss and deduct life. No eliminations happen in Round 1 - a player who
    // misses both questions simply carries only 1 life into Round 2.
    if let Some(contestant) = state.contestants.get_mut(player_id) {
        contestant.round1_misses += 1;
        contestant.round1_questions += 1;
    }

    deduct_life(state, player_id);

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
    state.last_pointer_id = None;

    // Pick the first active player in seat order (Player 1..10) for Round 2
    if let Some(first_id) = state.player_queue.iter().find(|id| {
        state
            .contestants
            .get(*id)
            .map(|c| !c.eliminated)
            .unwrap_or(false)
    }) {
        state.active_player_id = Some(first_id.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::games::one_of_ten::types::Contestant;
    use std::collections::HashMap;

    #[allow(dead_code)]
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
            round3_exclusive: false,
            past_questions: vec![],
            player_queue: vec!["player1".to_string()],
            active: true,
            buzzer_queue: vec![],
            last_pointer_id: None,
            last_answer_correct: None,
            last_correct_answer: None,
            waiting_for_presenter: false,
            deferred_action: None,
        }
    }
}
