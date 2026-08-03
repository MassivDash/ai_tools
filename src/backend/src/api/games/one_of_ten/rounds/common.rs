use crate::api::games::one_of_ten::types::{GameState, GameStateSnapshot, OutgoingMessage};

/// Award points to a player
pub fn award_points(state: &mut GameState, player_id: &str, amount: i32) {
    if let Some(contestant) = state.contestants.get_mut(player_id) {
        contestant.score += amount;
    }
}

/// Deduct a life from a player
pub fn deduct_life(state: &mut GameState, player_id: &str) {
    if let Some(contestant) = state.contestants.get_mut(player_id) {
        if contestant.lives > 0 {
            contestant.lives -= 1;
        }
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
        last_answer_correct: state.last_answer_correct,
        last_correct_answer: state.last_correct_answer.clone(),
        waiting_for_presenter: state.waiting_for_presenter,
        winner_id: state.winner_id.clone(),
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
        .filter(|c| !c.eliminated)
        .map(|c| c.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::games::one_of_ten::types::{Contestant, Question, Round};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn contestant(id: &str, lives: i32) -> Contestant {
        Contestant {
            id: id.to_string(),
            session_id: id.to_string(),
            name: id.to_string(),
            age: "25".to_string(),
            score: 0,
            online: true,
            ready: true,
            lives,
            round1_misses: 0,
            round1_questions: 0,
            eliminated: false,
        }
    }

    fn state_with(players: &[(&str, i32)]) -> GameState {
        let mut state = GameState::new();
        for (id, lives) in players {
            state
                .contestants
                .insert(id.to_string(), contestant(id, *lives));
            state.player_queue.push(id.to_string());
        }
        state
    }

    #[test]
    fn test_award_points_accumulates_and_ignores_unknown_players() {
        let mut state = state_with(&[("p1", 3)]);

        award_points(&mut state, "p1", 10);
        award_points(&mut state, "p1", 20);
        award_points(&mut state, "ghost", 100);

        assert_eq!(state.contestants["p1"].score, 30);
        assert_eq!(state.contestants.len(), 1, "no player was invented");
    }

    #[test]
    fn test_deduct_life_stops_at_zero_and_ignores_unknown_players() {
        let mut state = state_with(&[("p1", 1)]);

        deduct_life(&mut state, "p1");
        assert_eq!(state.contestants["p1"].lives, 0);

        // A player on zero lives cannot go negative
        deduct_life(&mut state, "p1");
        assert_eq!(state.contestants["p1"].lives, 0);

        deduct_life(&mut state, "ghost");
    }

    #[test]
    fn test_check_elimination_only_fires_at_zero_lives() {
        let mut state = state_with(&[("p1", 1), ("p2", 0)]);

        assert!(!check_elimination(&mut state, "p1"));
        assert!(!state.contestants["p1"].eliminated);

        assert!(check_elimination(&mut state, "p2"));
        assert!(state.contestants["p2"].eliminated);

        assert!(
            !check_elimination(&mut state, "ghost"),
            "an unknown player is never eliminated"
        );
    }

    #[test]
    fn test_reset_question_state_clears_question_and_timer() {
        let mut state = state_with(&[("p1", 3)]);
        state.current_question = Some(Question {
            text: "2 + 2?".to_string(),
            correct_answer: "4".to_string(),
            options: None,
        });
        state.timer_start = Some(123);

        reset_question_state(&mut state);

        assert!(state.current_question.is_none());
        assert!(state.timer_start.is_none());
    }

    #[test]
    fn test_create_state_snapshot_mirrors_the_game_state() {
        let mut state = state_with(&[("p1", 3), ("p2", 2)]);
        state.presenter_id = Some("presenter".to_string());
        state.presenter_online = true;
        state.round = Round::Round2;
        state.active_player_id = Some("p1".to_string());
        state.timer_start = Some(999);
        state.decision_pending = true;
        state.last_answer_correct = Some(false);
        state.last_correct_answer = Some("4".to_string());
        state.waiting_for_presenter = true;
        state.winner_id = Some("p1".to_string());
        state.current_question = Some(Question {
            text: "2 + 2?".to_string(),
            correct_answer: "4".to_string(),
            options: Some(vec!["3".to_string(), "4".to_string()]),
        });

        let snapshot = create_state_snapshot(&state);

        assert!(snapshot.has_presenter);
        assert!(snapshot.presenter_online);
        assert_eq!(snapshot.contestants.len(), 2);
        assert_eq!(snapshot.round, Round::Round2);
        assert_eq!(snapshot.active_player_id, Some("p1".to_string()));
        assert_eq!(snapshot.timer_start, Some(999));
        assert!(snapshot.decision_pending);
        assert_eq!(snapshot.last_answer_correct, Some(false));
        assert_eq!(snapshot.last_correct_answer, Some("4".to_string()));
        assert!(snapshot.waiting_for_presenter);
        assert_eq!(snapshot.winner_id, Some("p1".to_string()));
        assert_eq!(
            snapshot
                .current_question
                .expect("expected a question")
                .correct_answer,
            "4"
        );

        // A lobby without a presenter reports has_presenter = false
        let empty = create_state_snapshot(&GameState::new());
        assert!(!empty.has_presenter);
        assert!(empty.contestants.is_empty());
        assert_eq!(empty.round, Round::Lobby);
    }

    #[test]
    fn test_create_state_update_wraps_a_snapshot() {
        let state = state_with(&[("p1", 3)]);

        match create_state_update(&state) {
            OutgoingMessage::StateUpdate(snapshot) => {
                assert_eq!(snapshot.contestants.len(), 1);
                assert_eq!(snapshot.contestants[0].id, "p1");
            }
            other => panic!("Expected a StateUpdate, got {:?}", other),
        }
    }

    #[test]
    fn test_active_contestant_helpers_ignore_eliminated_players() {
        let mut state = state_with(&[("p1", 3), ("p2", 3), ("p3", 0)]);
        state.contestants.get_mut("p3").unwrap().eliminated = true;

        assert_eq!(count_active_contestants(&state), 2);

        let mut ids = get_active_contestant_ids(&state);
        ids.sort();
        assert_eq!(ids, vec!["p1", "p2"]);

        assert_eq!(count_active_contestants(&GameState::new()), 0);
        assert!(get_active_contestant_ids(&GameState::new()).is_empty());
    }

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
