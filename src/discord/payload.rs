use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Payload {
    #[serde(flatten)]
    pub cmd: Commands,
    pub nonce: Uuid,
}

impl Payload {
    pub fn new(cmd: Commands) -> Self {
        Self {
            cmd,
            nonce: Uuid::new_v4(),
        }
    }
}

impl Into<serde_json::Value> for Payload {
    fn into(self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "cmd", content = "args")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Commands {
    Authorize {
        client_id: String,
        scopes: Vec<&'static str>,
    },
    Authenticate {
        access_token: String,
    },
    SetVoiceSettings {
        mute: bool,
        deaf: bool,
    },
}
