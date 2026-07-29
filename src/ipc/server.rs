use std::path::Path;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    sync::{broadcast, mpsc},
};

use crate::{
    SharedState,
    discord::{EventData, events::User},
    ipc::Command,
};

// TODO: Probably have this behind an env variable
static SOCKET_PATH: &str = "/run/user/1000/discord-bridge";

pub async fn run_server(
    state: SharedState,
    broadcast_rx: broadcast::Receiver<EventData>,
    mpsc_tx: mpsc::Sender<Command>,
    user: User,
) -> ! {
    if Path::new(SOCKET_PATH).exists() {
        let _ = tokio::fs::remove_file(SOCKET_PATH).await;
    }

    let listener = UnixListener::bind(SOCKET_PATH)
        .inspect_err(|e| {
            tracing::error!(path = SOCKET_PATH, error = %e, "Failed to bind to socket");
        })
        .unwrap();

    tracing::info!(path = SOCKET_PATH, "Listening for connections");

    loop {
        match listener.accept().await {
            Ok((mut stream, addr)) => {
                let mpsc_tx = mpsc_tx.clone();
                let broadcast_rx = broadcast_rx.resubscribe();

                let channel_id = { state.read().await.channel_id.clone() };

                let user_json = serde_json::json!({
                    "evt": "INIT",
                    "data": {
                        "user": user,
                        "channel_id": channel_id
                    }
                });

                let _ = stream.write_all(user_json.to_string().as_bytes()).await;
                let _ = stream.write_all(b"\n").await;
                let _ = stream.flush().await;

                tokio::spawn(async move {
                    tracing::info!(peer = ?addr, "Accepted connection");
                    if let Err(e) = handle_client(stream, mpsc_tx, broadcast_rx).await {
                        tracing::error!(error = %e, "Error handling connection");
                    }
                });
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to accept connection");
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum ClientError {
    #[error("Failed to read from client: {0}")]
    Read(std::io::Error),
    #[error("Failed to write to client: {0}")]
    Write(std::io::Error),
    #[error("Failed to send command to main loop: {0}")]
    SendCommand(#[from] mpsc::error::SendError<Command>),
    #[error("Failed to serialize event data: {0}")]
    SerializeEvent(#[from] serde_json::Error),
}

async fn handle_client(
    stream: UnixStream,
    mpsc_tx: mpsc::Sender<Command>,
    mut broadcast_rx: broadcast::Receiver<EventData>,
) -> Result<(), ClientError> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    loop {
        tokio::select! {
            line = lines.next_line() => {
                match line.map_err(ClientError::Read)? {
                    Some(raw) => {
                        let Ok(cmd) = serde_json::from_str::<Command>(&raw).inspect_err(|e| {
                            tracing::warn!(error = %e, "Invalid client message");
                        }) else {
                            continue;
                        };

                        mpsc_tx.send(cmd).await?;
                    },
                    None => break,
                }
            }

            event = broadcast_rx.recv() => {
                match event {
                    Ok(data) => {
                        let mut msg = serde_json::to_vec(&data)?;
                        msg.push(b'\n');
                        writer.write_all(&msg).await.map_err(ClientError::Write)?;
                    }
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        tracing::warn!(count, "Client lagged behind broadcasts");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    Ok(())
}
