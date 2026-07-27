use serde::{Deserialize, Serialize};
use strum::EnumDiscriminants;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, EnumDiscriminants)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(tag = "evt", content = "data")]
#[strum_discriminants(name(Event))]
#[strum_discriminants(derive(Serialize, Deserialize))]
#[strum_discriminants(serde(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum EventData {
    VoiceSettingsUpdate {
        mute: bool,
        deaf: bool,
    },
    VoiceChannelSelect {
        channel_id: Option<String>,
        guild_id: Option<String>,
    },
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
