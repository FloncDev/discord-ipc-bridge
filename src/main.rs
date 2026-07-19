use discord_ipc_bridge::discord::Client;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let client_id = std::env::var("CLIENT_ID").expect("CLIENT_ID not set");
    let client_secret = std::env::var("CLIENT_SECRET").expect("CLIENT_SECRET not set");

    let client = Client::connect(&client_id, &client_secret)
        .await
        .expect("Failed to connect to Discord IPC");

    let (mut discord_rx, mut discord_tx) = client.to_split();
    let (cmd_tx, cmd_rx) = mpsc::channel(32);
}
