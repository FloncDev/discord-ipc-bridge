use std::sync::Arc;

use discord_ipc_bridge::{
    State,
    discord::Client,
    ipc::{Command, EventData, command::Event},
};
use tokio::sync::{RwLock, mpsc};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let client_id = std::env::var("CLIENT_ID").expect("CLIENT_ID not set");
    let client_secret = std::env::var("CLIENT_SECRET").expect("CLIENT_SECRET not set");

    let client = Client::connect(&client_id, &client_secret)
        .await
        .expect("Failed to connect to Discord IPC");

    let (cmd_tx, cmd_rx) = mpsc::channel::<Command>(32);
    let (broadcast_tx, mut broadcast_rx) = tokio::sync::broadcast::channel::<EventData>(32);
    let state = Arc::new(RwLock::new(State::new()));

    // Run the client
    let client_handle = tokio::spawn(async move {
        client.run(state, broadcast_tx, cmd_rx).await;
    });

    let _ = cmd_tx
        .send(Command::Subscribe(Event::VoiceSettingsUpdate))
        .await;
    let _ = cmd_tx.send(Command::ToggleMute).await;

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl+C");
}
