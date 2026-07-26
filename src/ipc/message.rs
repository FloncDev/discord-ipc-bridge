use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventData {
    VoiceSettingsUpdate { is_muted: bool, is_deafened: bool },
}
