use crate::api::games::one_of_fifteen::types::{GameState, Round};
use rand::seq::SliceRandom;

/// Select the next active player based on round rules
/// Returns None if no eligible players exist
pub fn select_next_player(
    state: &GameState,
    current_id: Option<&str>,
    round: &Round,
) -> Option<String> {
    let active_ids: Vec<String> = state
        .contestants
        .values()
        .filter(|c| !c.eliminated && c.online)
        .map(|c| c.id.clone())
        .collect();

    if active_ids.is_empty() {
        return None;
    }

    match round {
        Round::Round1 => {
            // Round 1: Rotate through players who haven't completed 2 questions
            let incomplete: Vec<String> = active_ids
                .iter()
                .filter(|id| {
                    state
                        .contestants
                        .get(*id)
                        .map(|c| c.round1_questions < 2)
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
        .filter(|c| !c.eliminated && c.online)
        .map(|c| c.id.clone())
        .collect();

    if active_ids.is_empty() {
        return None;
    }

    let mut rng = rand::rng();
    active_ids.shuffle(&mut rng);
    active_ids.first().cloned()
}
