use crate::api::games::one_of_ten::types::{GameState, Round};
use rand::seq::SliceRandom;

/// Select the next active player based on round rules
/// Returns None if no eligible players exist
pub fn select_next_player(
    state: &GameState,
    current_id: Option<&str>,
    round: &Round,
) -> Option<String> {
    match round {
        Round::Round1 => {
            // Round 1: Rotate through players who haven't completed 2 questions,
            // in seat order (the order players joined - Player 1 .. Player 10),
            // not the arbitrary order HashMap iteration would give us.
            let incomplete: Vec<String> = state
                .player_queue
                .iter()
                .filter(|id| {
                    state
                        .contestants
                        .get(*id)
                        .map(|c| !c.eliminated && c.round1_questions < 2)
                        .unwrap_or(false)
                })
                .cloned()
                .collect();

            if incomplete.is_empty() {
                return None; // Round 1 complete
            }

            // If current player is provided, try to find next in rotation
            if let Some(curr) = current_id {
                if let Some(pos) = incomplete.iter().position(|id| id == curr) {
                    let next_pos = (pos + 1) % incomplete.len();
                    return Some(incomplete[next_pos].clone());
                }
            }

            // Otherwise, pick first incomplete player
            incomplete.first().cloned()
        }
        Round::Round2 | Round::Round3 => {
            // Round 2 & 3: pointing/buzzer based, don't auto-select
            // Caller should handle selection explicitly
            None
        }
        Round::Lobby | Round::Finished => None,
    }
}

/// Select a random active (non-eliminated, online) player
pub fn select_random_active(state: &GameState) -> Option<String> {
    let mut active_ids: Vec<String> = state
        .contestants
        .values()
        .filter(|c| !c.eliminated)
        .map(|c| c.id.clone())
        .collect();

    if active_ids.is_empty() {
        return None;
    }

    let mut rng = rand::rng();
    active_ids.shuffle(&mut rng);
    active_ids.first().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::games::one_of_ten::types::Contestant;

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

    /// Builds a lobby holding `ids` in seat order.
    fn state_with(ids: &[&str]) -> GameState {
        let mut state = GameState::new();
        for id in ids {
            state.contestants.insert(id.to_string(), contestant(id));
            state.player_queue.push(id.to_string());
        }
        state
    }

    #[test]
    fn test_round1_picks_the_first_incomplete_player_without_a_current_player() {
        let state = state_with(&["p1", "p2", "p3"]);

        assert_eq!(
            select_next_player(&state, None, &Round::Round1),
            Some("p1".to_string())
        );
    }

    #[test]
    fn test_round1_rotates_in_seat_order_and_wraps_around() {
        let state = state_with(&["p1", "p2", "p3"]);

        assert_eq!(
            select_next_player(&state, Some("p1"), &Round::Round1),
            Some("p2".to_string())
        );
        assert_eq!(
            select_next_player(&state, Some("p2"), &Round::Round1),
            Some("p3".to_string())
        );
        assert_eq!(
            select_next_player(&state, Some("p3"), &Round::Round1),
            Some("p1".to_string()),
            "the rotation wraps back to the first seat"
        );
    }

    #[test]
    fn test_round1_skips_eliminated_and_finished_players() {
        let mut state = state_with(&["p1", "p2", "p3"]);
        state.contestants.get_mut("p2").unwrap().eliminated = true;
        // p3 already answered both of its questions
        state.contestants.get_mut("p3").unwrap().round1_questions = 2;

        assert_eq!(
            select_next_player(&state, Some("p1"), &Round::Round1),
            Some("p1".to_string()),
            "p1 is the only eligible player, so the rotation stays on it"
        );
        assert_eq!(
            select_next_player(&state, None, &Round::Round1),
            Some("p1".to_string())
        );
    }

    #[test]
    fn test_round1_falls_back_to_the_first_seat_for_an_unknown_current_player() {
        let state = state_with(&["p1", "p2"]);

        assert_eq!(
            select_next_player(&state, Some("ghost"), &Round::Round1),
            Some("p1".to_string())
        );
    }

    #[test]
    fn test_round1_returns_none_when_every_player_is_done() {
        let mut state = state_with(&["p1", "p2"]);
        for id in ["p1", "p2"] {
            state.contestants.get_mut(id).unwrap().round1_questions = 2;
        }

        assert_eq!(select_next_player(&state, Some("p1"), &Round::Round1), None);

        // An empty lobby is "complete" too
        assert_eq!(
            select_next_player(&GameState::new(), None, &Round::Round1),
            None
        );
    }

    #[test]
    fn test_round1_ignores_queue_entries_without_a_contestant_record() {
        let mut state = state_with(&["p1"]);
        state.player_queue.push("ghost".to_string());

        assert_eq!(
            select_next_player(&state, Some("p1"), &Round::Round1),
            Some("p1".to_string())
        );
    }

    #[test]
    fn test_other_rounds_never_auto_select() {
        let state = state_with(&["p1", "p2"]);

        for round in [Round::Round2, Round::Round3, Round::Lobby, Round::Finished] {
            assert_eq!(
                select_next_player(&state, Some("p1"), &round),
                None,
                "{:?} should not auto-select a player",
                round
            );
        }
    }

    #[test]
    fn test_select_random_active_only_returns_a_non_eliminated_player() {
        let mut state = state_with(&["p1", "p2", "p3"]);
        state.contestants.get_mut("p1").unwrap().eliminated = true;
        // `online` is deliberately not part of the filter
        state.contestants.get_mut("p2").unwrap().online = false;

        for _ in 0..20 {
            let picked = select_random_active(&state).expect("expected a player");
            assert!(
                picked == "p2" || picked == "p3",
                "eliminated players must never be picked, got {}",
                picked
            );
        }
    }

    #[test]
    fn test_select_random_active_with_a_single_survivor_is_deterministic() {
        let mut state = state_with(&["p1", "p2"]);
        state.contestants.get_mut("p2").unwrap().eliminated = true;

        assert_eq!(select_random_active(&state), Some("p1".to_string()));
    }

    #[test]
    fn test_select_random_active_returns_none_without_survivors() {
        let mut state = state_with(&["p1"]);
        state.contestants.get_mut("p1").unwrap().eliminated = true;

        assert_eq!(select_random_active(&state), None);
        assert_eq!(select_random_active(&GameState::new()), None);
    }
}
