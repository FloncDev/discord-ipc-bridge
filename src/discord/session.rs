use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Session {
    pub access_token: String,
    expires_at: DateTime<Utc>,
    refresh_token: String,
}

#[derive(Serialize, Deserialize)]
pub struct TokenResponse {
    access_token: String,
    expires_in: i64,
    refresh_token: String,
}

#[derive(thiserror::Error, Debug)]
pub enum OAuthError {
    #[error("Failed to exchange auth code for token: {0}")]
    TokenExchange(#[from] reqwest::Error),
    #[error("Failed to parse token response: {0}")]
    TokenParse(#[from] serde_json::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum CacheError {
    #[error("Failed to read cache file: {0}")]
    Read(#[from] std::io::Error),
    #[error("Failed to parse cache file: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("Failed to exchange refresh token for new token: {0}")]
    TokenExchange(#[from] reqwest::Error),
}

impl Session {
    pub async fn from_auth_code(
        auth_code: String,
        client_id: &str,
        client_secret: &str,
    ) -> Result<Session, OAuthError> {
        let client = reqwest::Client::new();

        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("grant_type", "authorization_code"),
            ("code", &auth_code),
            ("redirect_uri", "http://127.0.0.1"),
        ];

        let response: TokenResponse = client
            .post("https://discord.com/api/oauth2/token")
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        let expires_at = Utc::now() + chrono::Duration::seconds(response.expires_in);

        let session = Session {
            access_token: response.access_token,
            expires_at,
            refresh_token: response.refresh_token,
        };

        // TODO: Cache session

        Ok(session)
    }

    pub async fn from_cache(client_id: &str, client_secret: &str) -> Result<Session, CacheError> {
        // Try to read from cache file
        let cache_path = std::env::var("DISCORD_SESSION_CACHE").unwrap_or_else(|_| ".".to_string());
        let cache_file_path = std::path::Path::new(&cache_path).join("session_cache.json");

        let file = tokio::fs::File::open(cache_file_path).await?;
        let session: Session = serde_json::from_reader(file.into_std().await)?;

        // Check to see if its expired
        if session.expires_at > Utc::now() {
            return Ok(session);
        }

        tracing::info!("Token expired, refreshing");

        // Get new session from refresh token
        let client = reqwest::Client::new();
        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("grant_type", "refresh_token"),
            ("refresh_token", &session.refresh_token),
        ];

        let response: TokenResponse = client
            .post("https://discord.com/api/oauth2/token")
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        let expires_at = Utc::now() + chrono::Duration::seconds(response.expires_in);

        Ok(Session {
            access_token: response.access_token,
            expires_at,
            refresh_token: response.refresh_token,
        })
    }
}
