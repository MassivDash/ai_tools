use crate::api::games::one_of_fifteen::types::GameState;
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
            round: crate::api::games::one_of_fifteen::types::Round::Lobby,
            active_player_id: None,
            player_queue: Vec::new(),
            current_question: None,
            past_questions: Vec::new(),
            timer_start: None,
            buzzer_queue: Vec::new(),
            last_pointer_id: None,
            decision_pending: false,
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}
