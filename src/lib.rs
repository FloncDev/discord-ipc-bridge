use std::sync::Arc;

use tokio::sync::RwLock;

pub mod discord;
pub mod ipc;

pub struct State {
    pub is_muted: bool,
    pub is_deafened: bool,
    pub channel_id: Option<String>,
    pub guild_id: Option<String>,
}

impl State {
    pub fn new() -> Self {
        Self {
            is_muted: false,
            is_deafened: false,
            channel_id: None,
            guild_id: None,
        }
    }

    pub fn audio_status(&self) -> (bool, bool) {
        (self.is_muted, self.is_deafened)
    }

    pub fn set_audio_status(&mut self, is_muted: bool, is_deafened: bool) {
        self.is_muted = is_muted;
        self.is_deafened = is_deafened;
    }

    pub fn voice_channel(&self) -> Option<(String, String)> {
        match (&self.channel_id, &self.guild_id) {
            (Some(channel_id), Some(guild_id)) => Some((channel_id.clone(), guild_id.clone())),
            _ => None,
        }
    }

    pub fn set_voice_channel(&mut self, channel_id: Option<String>, guild_id: Option<String>) {
        self.channel_id = channel_id;
        self.guild_id = guild_id;
    }
}

pub type SharedState = Arc<RwLock<State>>;
