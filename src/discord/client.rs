use serde_json::json;
use tokio::{
    net::UnixStream,
    sync::{broadcast, mpsc},
};

use crate::{
    SharedState,
    discord::{
        OAuthError, Response, ResponseCommands, Session,
        frames::{ReadError, WriteError, read_frame, write_frame},
        payload::{Commands, Payload},
    },
    ipc::{Command, EventData, command::Event},
};

pub struct Client {
    stream: UnixStream,
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
        let stream =
            UnixStream::connect("/var/folders/t8/8xvn4bvd3pg57t3qy12wm00c0000gn/T/discord-ipc-1")
                .await
                .map_err(ConnectionError::Io)?;
        tracing::info!("Connected to socket");

        let mut client = Client { stream };

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
                Session::from_auth_code(auth_code, client_id, client_secret).await?
            }
        };

        client.authenticate(session.access_token).await?;

        Ok(client)
    }

    async fn handshake(&mut self, client_id: &str) -> Result<(), ConnectionError> {
        let payload = json!({"v": 1, "client_id": client_id});
        self.write(0, payload).await?;

        // Ignore the initial READY event
        let (_, _) = self.read().await?;

        Ok(())
    }

    async fn authenticate(&mut self, access_token: String) -> Result<(), ConnectionError> {
        let command = Commands::Authenticate { access_token };

        self.write(1, Payload::new(command)).await?;

        // Ignore the response
        let (_, _) = self.read().await?;

        Ok(())
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

        loop {
            tokio::select! {
                res = read_frame(&mut discord_rx) => {
                    if let Ok(msg) = res {
                        // TODO: Parse message
                        tracing::info!("Received message from Discord: {:?}", msg);
                    }
                }

                Some(msg) = mpsc_rx.recv() => {
                    let payload: serde_json::Value = match msg {
                        Command::ToggleMute => {
                            let (is_muted, is_deafend) = state.read().await.audio_status();

                            Commands::SetVoiceSettings { mute: !is_muted, deaf: is_deafend }.as_payload().into()
                        },
                        Command::ToggleDeafen => {
                            let (is_muted, is_deafend) = state.read().await.audio_status();

                            Commands::SetVoiceSettings { mute: is_muted, deaf: !is_deafend }.as_payload().into()
                        },
                        Command::Subscribe(event) => {
                            event.subscribe()
                        },
                        Command::Unsubscribe(event) => {
                            event.unsubscribe()
                        }
                    };

                    write_frame(&mut discord_tx, 1, payload).await.expect("Failed to send command to Discord");
                }
            }
        }
    }
}
