pub mod ai;
pub mod player_selection;
pub mod rounds;
pub mod state;
pub mod types;
pub mod websocket;

use actix::Recipient;
use std::sync::{Arc, Mutex};
use websocket::BroadcastingMessage;

// Re-export main entry point
pub use types::GameState;
pub use websocket::one_of_ten_ws_route;

pub type BroadcastHandle = Arc<Mutex<Vec<Recipient<BroadcastingMessage>>>>;
