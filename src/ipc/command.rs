use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    ToggleMute,
    ToggleDeafen,
    GetVoiceStatus,
    Subscribe(Event),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    VoiceSettingsUpdate,
}
