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

/// Check if the timer has expired
pub fn is_timed_out(timer_start: Option<u64>, duration_seconds: u64) -> bool {
    if let Some(start_ts) = timer_start {
        if let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            return now.as_secs() > start_ts + duration_seconds;
        }
    }
    false
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_is_timed_out() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Not timed out (start time is current time)
        assert!(!is_timed_out(Some(now), 60));

        // Timed out (start time is 61 seconds ago)
        let past = now - 61;
        assert!(is_timed_out(Some(past), 60));

        // Not timed out (start time is 59 seconds ago)
        let recent = now - 59;
        assert!(!is_timed_out(Some(recent), 60));

        // No timer start should not time out
        assert!(!is_timed_out(None, 60));
    }
}
