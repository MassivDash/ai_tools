use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;
use tokio::sync::RwLock;

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

pub struct GoogleOAuthProvider {
    client_id: String,
    client_secret: String,
    refresh_token: String,
    access_token: RwLock<Option<(String, std::time::Instant)>>,
    pub http_client: reqwest::Client,
}

impl GoogleOAuthProvider {
    pub fn new() -> Result<Self> {
        let client_id = env::var("GOOGLE_CLIENT_ID").context("Missing GOOGLE_CLIENT_ID")?;
        let client_secret =
            env::var("GOOGLE_CLIENT_SECRET").context("Missing GOOGLE_CLIENT_SECRET")?;
        let refresh_token =
            env::var("GOOGLE_REFRESH_TOKEN").context("Missing GOOGLE_REFRESH_TOKEN")?;

        Ok(Self {
            client_id,
            client_secret,
            refresh_token,
            access_token: RwLock::new(None),
            http_client: reqwest::Client::new(),
        })
    }

    pub fn is_configured() -> bool {
        env::var("GOOGLE_CLIENT_ID").is_ok()
            && env::var("GOOGLE_CLIENT_SECRET").is_ok()
            && env::var("GOOGLE_REFRESH_TOKEN").is_ok()
    }

    pub async fn get_access_token(&self) -> Result<String> {
        // Check cache first
        {
            let cache = self.access_token.read().await;
            if let Some((token, expires_at)) = &*cache {
                if std::time::Instant::now() < *expires_at {
                    return Ok(token.clone());
                }
            }
        }

        // Fetch new token
        let res = self.http_client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("refresh_token", self.refresh_token.as_str()),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .context("Failed to send token refresh request")?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Token refresh failed with {}: {}",
                status,
                text
            ));
        }

        let token_data: TokenResponse =
            res.json().await.context("Failed to parse token response")?;
        let token = token_data.access_token;

        // Subtract 60 seconds as a buffer
        let expires_at = std::time::Instant::now()
            + std::time::Duration::from_secs(token_data.expires_in.saturating_sub(60));

        // Update cache
        let mut cache = self.access_token.write().await;
        *cache = Some((token.clone(), expires_at));

        Ok(token)
    }
}
