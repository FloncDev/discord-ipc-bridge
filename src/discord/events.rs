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
    Error {
        code: u32,
        message: String,
    },
    VoiceSettingsUpdate {
        mute: bool,
        deaf: bool,
    },
    VoiceChannelSelect {
        channel_id: Option<String>,
        guild_id: Option<String>,
    },
    SpeakingStart {
        user_id: String,
    },
    SpeakingStop {
        user_id: String,
    },
    VoiceStateCreate {
        voice_state: VoiceState,
        user: User,
    },
    VoiceStateUpdate {
        voice_state: VoiceState,
        user: User,
    },
    VoiceStateDelete {
        voice_state: VoiceState,
        user: User,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceState {
    mute: bool,
    deaf: bool,
    self_mute: bool,
    self_deaf: bool,
    suppress: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    id: String,
    avatar: String,
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

// Struct for subscribing to events that are linked to a channel
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventSubscribe {
    SpeakingStart,
    SpeakingStop,
    VoiceStateCreate,
    VoiceStateUpdate,
    VoiceStateDelete,
}

impl EventSubscribe {
    pub fn subscribe(&self, channel_id: &String) -> serde_json::Value {
        serde_json::json!({
            "cmd": "SUBSCRIBE",
            "args": {
                "channel_id": channel_id
            },
            "evt": self,
            "nonce": Uuid::new_v4()
        })
    }

    pub fn unsubscribe(&self, channel_id: &String) -> serde_json::Value {
        serde_json::json!({
            "cmd": "UNSUBSCRIBE",
            "args": {
                "channel_id": channel_id
            },
            "evt": self,
            "nonce": Uuid::new_v4()
        })
    }
}
