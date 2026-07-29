use std::sync::Arc;

use discord_ipc_bridge::{
    State,
    discord::{Client, EventData},
    ipc::{Command, server::run_server},
};
use tokio::sync::{RwLock, broadcast, mpsc};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let client_id = std::env::var("CLIENT_ID").expect("CLIENT_ID not set");
    let client_secret = std::env::var("CLIENT_SECRET").expect("CLIENT_SECRET not set");

    // TODO: Wait for IPC socket to be there, discord may not have launched yet
    let client = Client::connect(&client_id, &client_secret)
        .await
        .expect("Failed to connect to Discord IPC");

    let (mpsc_tx, mpsc_rx) = mpsc::channel::<Command>(32);
    let (broadcast_tx, broadcast_rx) = broadcast::channel::<EventData>(32);
    let state = Arc::new(RwLock::new(State::new()));

    // Spawn a listener to log broadcasted events
    // tokio::spawn(async move {
    //     let broadcast_rx = broadcast_tx.subscribe();
    //     while let Ok(event) = broadcast_rx.recv().await {
    //         tracing::debug!("Received broadcast event: {event:?}");
    //     }
    // });

    // Run the client
    let user = client.user.clone().unwrap();

    let cloned_state = state.clone();
    let client_handle = tokio::spawn(async move {
        client.run(cloned_state, broadcast_tx, mpsc_rx).await;
    });

    let ipc_handle = tokio::spawn(async move {
        run_server(state, broadcast_rx, mpsc_tx, user).await;
    });

    // let _ = cmd_tx.send(Command::ToggleMute).await;

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl+C");

    tracing::info!("Shutting down..");

    client_handle.abort();
    ipc_handle.abort();
}
