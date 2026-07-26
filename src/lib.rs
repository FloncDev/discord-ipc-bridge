use std::sync::Arc;

use tokio::sync::RwLock;

pub mod discord;
pub mod ipc;

pub struct State {
    pub is_muted: bool,
    pub is_deafened: bool,
    pub voice_id: Option<String>,
}

impl State {
    pub fn new() -> Self {
        Self {
            is_muted: false,
            is_deafened: false,
            voice_id: None,
        }
    }

    pub fn audio_status(&self) -> (bool, bool) {
        (self.is_muted, self.is_deafened)
    }
}

pub type SharedState = Arc<RwLock<State>>;
