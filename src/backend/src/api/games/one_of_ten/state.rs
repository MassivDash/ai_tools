use crate::api::games::one_of_ten::types::GameState;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type GameStateHandle = Arc<Mutex<GameState>>;

impl GameState {
    pub fn new() -> Self {
        Self {
            presenter_id: None,
            presenter_online: false,
            contestants: HashMap::new(),
            active: true,
            round: crate::api::games::one_of_ten::types::Round::Lobby,
            active_player_id: None,
            player_queue: Vec::new(),
            current_question: None,
            past_questions: Vec::new(),
            timer_start: None,
            buzzer_queue: Vec::new(),
            last_pointer_id: None,
            decision_pending: false,
            round3_exclusive: false,
            last_answer_correct: None,
            last_correct_answer: None,
            waiting_for_presenter: false,
            deferred_action: None,
            winner_id: None,
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::games::one_of_ten::types::Round;

    #[test]
    fn test_new_starts_an_empty_active_lobby() {
        let state = GameState::new();

        assert!(state.presenter_id.is_none());
        assert!(!state.presenter_online);
        assert!(state.contestants.is_empty());
        assert!(state.active, "a fresh game is marked active");
        assert_eq!(state.round, Round::Lobby);
        assert!(state.active_player_id.is_none());
        assert!(state.player_queue.is_empty());
        assert!(state.current_question.is_none());
        assert!(state.past_questions.is_empty());
        assert!(state.timer_start.is_none());
        assert!(state.buzzer_queue.is_empty());
        assert!(state.last_pointer_id.is_none());
        assert!(!state.decision_pending);
        assert!(!state.round3_exclusive);
        assert!(state.last_answer_correct.is_none());
        assert!(state.last_correct_answer.is_none());
        assert!(!state.waiting_for_presenter);
        assert!(state.deferred_action.is_none());
        assert!(state.winner_id.is_none());
    }

    #[test]
    fn test_default_matches_new() {
        let default = GameState::default();
        let new = GameState::new();

        assert_eq!(default.round, new.round);
        assert_eq!(default.active, new.active);
        assert_eq!(default.contestants.len(), new.contestants.len());
        assert_eq!(default.winner_id, new.winner_id);
    }

    #[test]
    fn test_state_handle_shares_one_game() {
        let handle: GameStateHandle = Arc::new(Mutex::new(GameState::new()));
        let clone = Arc::clone(&handle);

        clone.lock().unwrap().round = Round::Round1;

        assert_eq!(handle.lock().unwrap().round, Round::Round1);
    }
}
