use serde::{Deserialize, Serialize};

// use crate::discord::Event;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    ToggleMute,
    ToggleDeafen,
    // Subscribe(Event),
    // Unsubscribe(Event),
}
