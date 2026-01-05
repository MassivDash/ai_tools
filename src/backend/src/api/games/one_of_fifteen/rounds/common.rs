use crate::api::games::one_of_fifteen::types::{GameState, GameStateSnapshot, OutgoingMessage};

/// Award points to a player
pub fn award_points(state: &mut GameState, player_id: &str, amount: i32) {
    if let Some(contestant) = state.contestants.get_mut(player_id) {
        contestant.score += amount;
    }
}

/// Deduct a life from a player
pub fn deduct_life(state: &mut GameState, player_id: &str) {
    if let Some(contestant) = state.contestants.get_mut(player_id) {
        contestant.lives = contestant.lives.saturating_sub(1);
    }
}

/// Check if a player should be eliminated (no lives left)
pub fn check_elimination(state: &mut GameState, player_id: &str) -> bool {
    if let Some(contestant) = state.contestants.get_mut(player_id) {
        if contestant.lives <= 0 {
            contestant.eliminated = true;
            return true;
        }
    }
    false
}

/// Reset question-related state
pub fn reset_question_state(state: &mut GameState) {
    state.current_question = None;
    state.timer_start = None;
}

/// Create a state snapshot for broadcasting
pub fn create_state_snapshot(state: &GameState) -> GameStateSnapshot {
    GameStateSnapshot {
        has_presenter: state.presenter_id.is_some(),
        presenter_online: state.presenter_online,
        contestants: state.contestants.values().cloned().collect(),
        round: state.round.clone(),
        active_player_id: state.active_player_id.clone(),
        current_question: state.current_question.clone(),
        timer_start: state.timer_start,
        decision_pending: state.decision_pending,
    }
}

/// Create a state update message
pub fn create_state_update(state: &GameState) -> OutgoingMessage {
    OutgoingMessage::StateUpdate(create_state_snapshot(state))
}

/// Count active (non-eliminated) contestants
pub fn count_active_contestants(state: &GameState) -> usize {
    state.contestants.values().filter(|c| !c.eliminated).count()
}

/// Get all active contestant IDs
pub fn get_active_contestant_ids(state: &GameState) -> Vec<String> {
    state
        .contestants
        .values()
        .filter(|c| !c.eliminated && c.online)
        .map(|c| c.id.clone())
        .collect()
}
