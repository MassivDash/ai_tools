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
    // Track miss and deduct life. If a player misses both questions in Round 1, they are eliminated.
    if let Some(contestant) = state.contestants.get_mut(player_id) {
        contestant.round1_misses += 1;
        contestant.round1_questions += 1;
        if contestant.round1_misses >= 2 {
            contestant.eliminated = true;
            contestant.lives = 0;
        } else if contestant.lives > 0 {
            contestant.lives -= 1;
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
    // Track miss and deduct life. If a player misses both questions in Round 1, they are eliminated.
    if let Some(contestant) = state.contestants.get_mut(player_id) {
        contestant.round1_misses += 1;
        contestant.round1_questions += 1;
        if contestant.round1_misses >= 2 {
            contestant.eliminated = true;
            contestant.lives = 0;
        } else if contestant.lives > 0 {
            contestant.lives -= 1;
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

    fn contestant(id: &str) -> Contestant {
        Contestant {
            id: id.to_string(),
            session_id: id.to_string(),
            name: id.to_string(),
            age: "25".to_string(),
            score: 0,
            online: true,
            ready: true,
            lives: 3,
            round1_misses: 0,
            round1_questions: 0,
            eliminated: false,
        }
    }

    /// A Round 1 game with `ids` seated in order and the first one on the clock.
    fn state_with(ids: &[&str]) -> GameState {
        let mut state = GameState::new();
        state.presenter_id = Some("presenter".to_string());
        state.presenter_online = true;
        state.round = Round::Round1;
        for id in ids {
            state.contestants.insert(id.to_string(), contestant(id));
            state.player_queue.push(id.to_string());
        }
        state.active_player_id = ids.first().map(|id| id.to_string());
        state
    }

    fn snapshot_of(
        messages: &[OutgoingMessage],
    ) -> &crate::api::games::one_of_ten::types::GameStateSnapshot {
        assert_eq!(messages.len(), 1, "expected exactly one state update");
        match &messages[0] {
            OutgoingMessage::StateUpdate(snapshot) => snapshot,
            other => panic!("Expected a StateUpdate, got {:?}", other),
        }
    }

    #[test]
    fn test_correct_answer_counts_the_question_and_advances_the_rotation() {
        let mut state = state_with(&["p1", "p2"]);
        state.current_question = Some(crate::api::games::one_of_ten::types::Question {
            text: "2 + 2?".to_string(),
            correct_answer: "4".to_string(),
            options: None,
        });
        state.timer_start = Some(42);

        let messages = handle_correct_answer(&mut state, "p1");

        let p1 = &state.contestants["p1"];
        assert_eq!(p1.round1_questions, 1);
        assert_eq!(p1.round1_misses, 0);
        assert_eq!(p1.lives, 3, "a correct answer costs nothing");
        assert_eq!(p1.score, 0, "scoring is disabled in round 1");
        assert!(state.current_question.is_none());
        assert!(state.timer_start.is_none());
        assert_eq!(state.active_player_id, Some("p2".to_string()));
        assert_eq!(state.round, Round::Round1);

        let snapshot = snapshot_of(&messages);
        assert_eq!(snapshot.active_player_id, Some("p2".to_string()));
        assert_eq!(snapshot.round, Round::Round1);
    }

    #[test]
    fn test_wrong_answer_costs_a_life_and_advances_the_rotation() {
        let mut state = state_with(&["p1", "p2"]);

        let messages = handle_wrong_answer(&mut state, "p1");

        let p1 = &state.contestants["p1"];
        assert_eq!(p1.round1_misses, 1);
        assert_eq!(p1.round1_questions, 1);
        assert_eq!(p1.lives, 2);
        assert!(!p1.eliminated);
        assert_eq!(state.active_player_id, Some("p2".to_string()));
        assert_eq!(snapshot_of(&messages).contestants.len(), 2);
    }

    #[test]
    fn test_timeout_is_scored_like_a_wrong_answer() {
        let mut state = state_with(&["p1", "p2"]);
        state.timer_start = Some(42);

        handle_timeout(&mut state, "p1");
        {
            let p1 = &state.contestants["p1"];
            assert_eq!(p1.round1_misses, 1);
            assert_eq!(p1.round1_questions, 1);
            assert_eq!(p1.lives, 2);
            assert!(!p1.eliminated);
        }
        assert!(state.timer_start.is_none());
        assert_eq!(state.active_player_id, Some("p2".to_string()));

        // Two timeouts eliminate the player outright
        handle_timeout(&mut state, "p1");
        let p1 = &state.contestants["p1"];
        assert_eq!(p1.round1_misses, 2);
        assert_eq!(p1.lives, 0);
        assert!(p1.eliminated);
    }

    #[test]
    fn test_a_timeout_on_the_last_question_transitions_to_round2() {
        let mut state = state_with(&["p1", "p2"]);
        for id in ["p1", "p2"] {
            state.contestants.get_mut(id).unwrap().round1_questions = 1;
        }
        handle_timeout(&mut state, "p1");
        assert_eq!(state.round, Round::Round1);

        let messages = handle_timeout(&mut state, "p2");

        assert_eq!(state.round, Round::Round2);
        assert_eq!(state.active_player_id, Some("p1".to_string()));
        assert_eq!(snapshot_of(&messages).round, Round::Round2);
    }

    #[test]
    fn test_a_player_with_no_lives_left_is_not_pushed_negative() {
        let mut state = state_with(&["p1", "p2"]);
        state.contestants.get_mut("p1").unwrap().lives = 0;

        handle_wrong_answer(&mut state, "p1");

        let p1 = &state.contestants["p1"];
        assert_eq!(p1.lives, 0);
        assert_eq!(p1.round1_misses, 1);
        assert!(!p1.eliminated, "one miss is not enough to eliminate");
    }

    #[test]
    fn test_handlers_ignore_an_unknown_player_but_still_broadcast() {
        let mut state = state_with(&["p1"]);

        let messages = handle_wrong_answer(&mut state, "ghost");

        assert_eq!(state.contestants["p1"].round1_questions, 0);
        assert_eq!(snapshot_of(&messages).contestants.len(), 1);
    }

    #[test]
    fn test_check_round1_complete() {
        let mut state = state_with(&["p1", "p2"]);
        assert!(!check_round1_complete(&state));

        state.contestants.get_mut("p1").unwrap().round1_questions = 2;
        assert!(
            !check_round1_complete(&state),
            "p2 has not answered anything yet"
        );

        state.contestants.get_mut("p2").unwrap().round1_questions = 2;
        assert!(check_round1_complete(&state));

        // Eliminated players do not hold the round open
        let mut state = state_with(&["p1", "p2"]);
        state.contestants.get_mut("p1").unwrap().round1_questions = 2;
        state.contestants.get_mut("p2").unwrap().eliminated = true;
        assert!(check_round1_complete(&state));

        // An empty lobby counts as complete
        assert!(check_round1_complete(&GameState::new()));
    }

    #[test]
    fn test_finishing_the_last_question_transitions_to_round2() {
        let mut state = state_with(&["p1", "p2"]);
        for id in ["p1", "p2"] {
            state.contestants.get_mut(id).unwrap().round1_questions = 1;
        }
        state.decision_pending = true;
        state.last_pointer_id = Some("p2".to_string());

        // p1 finishes first, so the round is still open
        handle_correct_answer(&mut state, "p1");
        assert_eq!(state.round, Round::Round1);
        assert_eq!(state.active_player_id, Some("p2".to_string()));

        // p2's answer completes the round
        let messages = handle_correct_answer(&mut state, "p2");

        assert_eq!(state.round, Round::Round2);
        assert_eq!(
            state.active_player_id,
            Some("p1".to_string()),
            "round 2 starts with the first surviving seat"
        );
        assert!(!state.decision_pending);
        assert!(state.last_pointer_id.is_none());
        assert_eq!(snapshot_of(&messages).round, Round::Round2);
    }

    #[test]
    fn test_transition_to_round2_skips_eliminated_seats() {
        let mut state = state_with(&["p1", "p2", "p3"]);
        state.contestants.get_mut("p1").unwrap().eliminated = true;

        transition_to_round2(&mut state);

        assert_eq!(state.round, Round::Round2);
        assert_eq!(state.active_player_id, Some("p2".to_string()));
    }

    #[test]
    fn test_transition_to_round2_without_survivors_leaves_no_active_player() {
        let mut state = state_with(&["p1"]);
        state.contestants.get_mut("p1").unwrap().eliminated = true;

        transition_to_round2(&mut state);

        assert_eq!(state.round, Round::Round2);
        assert!(state.active_player_id.is_none());
    }

    #[test]
    fn test_eliminating_the_last_player_ends_round1_immediately() {
        let mut state = state_with(&["p1"]);
        state.contestants.get_mut("p1").unwrap().round1_misses = 1;

        handle_wrong_answer(&mut state, "p1");

        assert!(state.contestants["p1"].eliminated);
        assert_eq!(
            state.round,
            Round::Round2,
            "with nobody left the round is complete and hands over to round 2"
        );
        assert!(state.active_player_id.is_none());
    }

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
            winner_id: None,
        }
    }

    #[test]
    fn test_two_strikes_elimination() {
        let mut state = create_test_state();

        handle_wrong_answer(&mut state, "player1");
        {
            let p = state.contestants.get("player1").unwrap();
            assert_eq!(p.round1_misses, 1);
            assert_eq!(p.round1_questions, 1);
            assert_eq!(p.lives, 2);
            assert_eq!(p.eliminated, false);
        }

        handle_wrong_answer(&mut state, "player1");
        {
            let p = state.contestants.get("player1").unwrap();
            assert_eq!(p.round1_misses, 2);
            assert_eq!(p.round1_questions, 2);
            assert_eq!(p.lives, 0);
            assert_eq!(p.eliminated, true);
        }
    }
}
