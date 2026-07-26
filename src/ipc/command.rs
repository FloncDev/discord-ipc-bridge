use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    ToggleMute,
    ToggleDeafen,
    Subscribe(Event),
    Unsubscribe(Event),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Event {
    VoiceSettingsUpdate,
}

impl Event {
    pub fn subscribe(&self) -> serde_json::Value {
        serde_json::json!({
            "cmd": "SUBSCRIBE",
            "evt": self,
            "nonce": Uuid::new_v4()
        })
    }

    pub fn unsubscribe(&self) -> serde_json::Value {
        serde_json::json!({
            "cmd": "UNSUBSCRIBE",
            "evt": self,
            "nonce": Uuid::new_v4()
        })
    }
}
