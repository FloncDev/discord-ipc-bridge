use serde_json::json;
use tokio::{
    net::UnixStream,
    sync::{broadcast, mpsc},
};

use crate::{
    SharedState,
    discord::{
        Event, EventData, OAuthError, Response, ResponseCommands, Session,
        events::{EventSubscribe, User},
        frames::{ReadError, WriteError, read_frame, write_frame},
        payload::{Commands, Payload},
        session::CacheWriteError,
    },
    ipc::Command,
};

pub struct Client {
    stream: UnixStream,
    pub user: Option<User>,
}

#[derive(thiserror::Error, Debug)]
pub enum ConnectionError {
    #[error("Failed to connect to Discord IPC: {0}")]
    Io(std::io::Error),
    #[error("Failed to read handshake response: {0}")]
    HandshakeRead(std::io::Error),
    #[error("Invalid handshake response: {0}")]
    InvalidHandshake(String),
    #[error("Failed to read from stream: {0}")]
    Read(#[from] ReadError),
    #[error("Failed to write to stream: {0}")]
    Write(#[from] WriteError),
    #[error("Failed to authenticate: {0}")]
    Authenticate(#[from] OAuthError),
    #[error("Failed to authorize: {0}")]
    Authorize(#[from] AuthorizationError),
    #[error("Failed to write Cache: {0}")]
    CacheWrite(#[from] CacheWriteError),
    #[error("Failed to parse response: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("Failed to get user")]
    GetUser,
}

#[derive(thiserror::Error, Debug)]
pub enum AuthorizationError {
    #[error("Failed to write authorization request: {0}")]
    Write(#[from] WriteError),
    #[error("Failed to read authorization response: {0}")]
    Read(#[from] ReadError),
    #[error("Failed to parse authorization response: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("Authorization failed: {0}")]
    Failed(String),
}

impl Client {
    pub async fn connect(client_id: &str, client_secret: &str) -> Result<Self, ConnectionError> {
        let stream = UnixStream::connect("/run/user/1000/discord-ipc-0")
            .await
            .map_err(ConnectionError::Io)?;
        tracing::info!("Connected to socket");

        let mut client = Client { stream, user: None };

        tracing::info!("Performing handshake");
        client.handshake(client_id).await?;

        // Check if we already have the token
        let session = match Session::from_cache(client_id, client_secret).await {
            Ok(session) => {
                tracing::info!("Found cached session");
                session
            }
            Err(_) => {
                tracing::info!("No cached session, requesting authorization");
                let auth_code = client.authorize(client_id).await?;

                tracing::info!("Getting access token");
                let session = Session::from_auth_code(auth_code, client_id, client_secret).await?;

                tracing::info!("Caching session");
                session.cache().await?;
                session
            }
        };

        client.user = Some(client.authenticate(session.access_token).await?);

        Ok(client)
    }

    async fn handshake(&mut self, client_id: &str) -> Result<(), ConnectionError> {
        let payload = json!({"v": 1, "client_id": client_id});
        self.write(0, payload).await?;

        // Ignore the initial READY event
        let (_, _) = self.read().await?;

        Ok(())
    }

    async fn authenticate(&mut self, access_token: String) -> Result<User, ConnectionError> {
        let command = Commands::Authenticate { access_token };

        self.write(1, Payload::new(command)).await?;

        // Ignore the response
        let (_, response) = self.read().await?;

        let response: Response = serde_json::from_value(response)?;

        match response.cmd {
            ResponseCommands::Authenticate { user } => Ok(user),
            _ => {
                return Err(ConnectionError::GetUser);
            }
        }
    }

    async fn authorize(&mut self, client_id: &str) -> Result<String, AuthorizationError> {
        let command = Commands::Authorize {
            client_id: client_id.to_string(),
            scopes: vec!["rpc", "identify", "rpc.voice.write", "rpc.voice.read"],
        };

        self.write(1, Payload::new(command)).await?;

        let (_, response) = self.read().await?;

        let response: Response = serde_json::from_value(response)?;

        match response.cmd {
            ResponseCommands::Authorize { code } => return Ok(code),
            _ => {
                return Err(AuthorizationError::Failed(format!(
                    "Unexpected response: {:?}",
                    response.cmd
                )));
            }
        }
    }

    pub fn to_split(
        self,
    ) -> (
        tokio::net::unix::OwnedReadHalf,
        tokio::net::unix::OwnedWriteHalf,
    ) {
        self.stream.into_split()
    }

    async fn read(&mut self) -> Result<(u32, serde_json::Value), ReadError> {
        read_frame(&mut self.stream).await
    }

    async fn write(
        &mut self,
        opcode: u32,
        payload: impl Into<serde_json::Value>,
    ) -> Result<(), WriteError> {
        write_frame(&mut self.stream, opcode, payload).await
    }
}

impl Client {
    pub async fn run(
        self,
        state: SharedState,
        broadcast_tx: broadcast::Sender<EventData>,
        mut mpsc_rx: mpsc::Receiver<Command>,
    ) {
        let (mut discord_rx, mut discord_tx) = self.to_split();

        // Subscribe to events
        let events = vec![Event::VoiceSettingsUpdate, Event::VoiceChannelSelect];
        for event in events {
            let payload = event.subscribe();
            write_frame(&mut discord_tx, 1, payload)
                .await
                .expect("Failed to subscribe to event");
        }

        write_frame(
            &mut discord_tx,
            1,
            Commands::GetSelectedVoiceChannel.as_payload(),
        )
        .await
        .expect("Failed to send GetSelectedVoiceChannel command to Discord");

        loop {
            tokio::select! {
                res = read_frame(&mut discord_rx) => {
                    if let Ok((_, msg)) = res {
                        // Check if it has a nonce and it isnt Null (it is a response to a command)
                        if msg.get("nonce").is_some() && msg.get("nonce").unwrap().is_string() {
                            // Parse it as a response
                            let Ok(response) = serde_json::from_value::<Response>(msg.clone()) else {
                                tracing::warn!("Failed to parse response: {:?}", msg);
                                continue;
                            };

                            match response.cmd {
                                ResponseCommands::GetSelectedVoiceChannel { channel_id, guild_id } => {
                                    if let Err(e) = handle_channel_change(state.clone(), &mut discord_tx, channel_id.clone(), guild_id.clone()).await {
                                        tracing::warn!("Failed to handle channel change: {:?}", e);
                                    }
                                },
                                ResponseCommands::Subscribe { event } => {
                                    tracing::info!("Subscribed to event: {:?}", event);
                                },
                                ResponseCommands::Unsubscribe { event } => {
                                    tracing::info!("Unsubscribed from event: {:?}", event);
                                },
                                _ => {
                                    tracing::warn!("Unexpected response: {:?}", response.cmd);
                                }
                            }

                            continue;
                        }

                        // Only parsing events
                        let Ok(event) = serde_json::from_value::<EventData>(msg.clone()) else {
                            tracing::warn!("Failed to parse event: {:?}", msg);
                            continue;
                        };

                        match &event {
                            EventData::Error { code, message } => {
                                tracing::warn!("Received error event: code={}, message={}", code, message);
                            },
                            EventData::VoiceSettingsUpdate { mute, deaf } => {
                                state.write().await.set_audio_status(*mute, *deaf);
                            },
                            EventData::VoiceChannelSelect { channel_id, guild_id } => {
                                if let Err(e) = handle_channel_change(state.clone(), &mut discord_tx, channel_id.clone(), guild_id.clone()).await {
                                    tracing::warn!("Failed to handle channel change: {:?}", e);
                                }
                            },
                            event => {
                                tracing::info!("Received event: {:?}", event);
                            }
                        };

                        // Log the state
                        let (is_muted, is_deafened) = state.read().await.audio_status();
                        let (channel_id, guild_id) = state.read().await.voice_channel().unwrap_or((String::from("None"), String::from("None")));
                        tracing::debug!("State updated: is_muted={}, is_deafened={}, channel_id={}, guild_id={}", is_muted, is_deafened, channel_id, guild_id);

                        match broadcast_tx.send(event) {
                            Ok(_) => {},
                            Err(e) => {
                                tracing::warn!("Failed to broadcast event: {:?}", e);
                            }
                        };

                    }
                }

                Some(msg) = mpsc_rx.recv() => {
                    let payload: serde_json::Value = match msg {
                        Command::ToggleMute => {
                            let (is_muted, _) = state.read().await.audio_status();

                            Commands::SetVoiceSettings { mute: Some(!is_muted), deaf: None }.as_payload().into()
                        },
                        Command::ToggleDeafen => {
                            let (_, is_deafend) = state.read().await.audio_status();

                            Commands::SetVoiceSettings { mute: None, deaf: Some(!is_deafend) }.as_payload().into()
                        },
                    };

                    write_frame(&mut discord_tx, 1, payload).await.expect("Failed to send command to Discord");
                }
            }
        }
    }
}

async fn handle_channel_change(
    state: SharedState,
    discord_tx: &mut tokio::net::unix::OwnedWriteHalf,
    new_channel_id: Option<String>,
    new_guild_id: Option<String>,
) -> Result<(), WriteError> {
    let events = vec![
        EventSubscribe::SpeakingStart,
        EventSubscribe::SpeakingStop,
        EventSubscribe::VoiceStateCreate,
        EventSubscribe::VoiceStateUpdate,
        EventSubscribe::VoiceStateDelete,
    ];

    let old_state = state.read().await.voice_channel();

    // Unsubscribe from the old channel if it exists
    if let Some((old_channel_id, _)) = old_state {
        for event in &events {
            let unsubscribe_payload = event.unsubscribe(&old_channel_id);
            write_frame(discord_tx, 1, unsubscribe_payload).await?;
        }
    }

    // Update the state with the new channel
    state
        .write()
        .await
        .set_voice_channel(new_channel_id.clone(), new_guild_id.clone());

    // Subscribe to the new channel if it exists
    if let (Some(channel_id), Some(_)) = (new_channel_id, new_guild_id) {
        for event in &events {
            let subscribe_payload = event.subscribe(&channel_id);
            write_frame(discord_tx, 1, subscribe_payload).await?;
        }
    }

    Ok(())
}
