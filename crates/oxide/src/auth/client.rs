use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use reqwest::{Client, StatusCode, multipart::Form};
use serde::Deserialize;
use tokio::time::sleep;

const DEVICE_ENDPOINT: &str = "https://id.twitch.tv/oauth2/device";
const TOKEN_ENDPOINT: &str = "https://id.twitch.tv/oauth2/token";
const VALIDATE_ENDPOINT: &str = "https://id.twitch.tv/oauth2/validate";
const DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";

pub struct AuthClient {
    http: Client,
}

#[derive(Deserialize)]
pub struct DeviceAuthorization {
    device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    #[serde(default)]
    pub scope: Vec<String>,
    #[serde(default)]
    pub token_type: String,
}

#[derive(Deserialize)]
pub struct ValidatedUser {
    pub login: String,
    pub user_id: String,
    pub client_id: String,
    pub scopes: Vec<String>,
    pub expires_in: u64,
}

#[derive(Deserialize)]
struct TwitchError {
    message: Option<String>,
    error: Option<String>,
}

impl AuthClient {
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            .user_agent(concat!("oxide/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("could not initialize the HTTP client")?;

        Ok(Self { http })
    }

    pub async fn begin_device_authorization(
        &self,
        client_id: &str,
        scopes: &[&str],
    ) -> Result<DeviceAuthorization> {
        let scope_list = scopes.join(" ");
        let form = Form::new()
            .text("client_id", client_id.to_owned())
            .text("scopes", scope_list);

        let response = self
            .http
            .post(DEVICE_ENDPOINT)
            .multipart(form)
            .send()
            .await
            .context("could not start Twitch device authorization")?;

        if !response.status().is_success() {
            bail!(
                "Twitch rejected the device authorization request (HTTP {})",
                response.status()
            );
        }

        response
            .json::<DeviceAuthorization>()
            .await
            .context("Twitch returned an invalid device authorization response")
    }

    pub async fn wait_for_device_token(
        &self,
        client_id: &str,
        scopes: &[&str],
        authorization: &DeviceAuthorization,
    ) -> Result<TokenResponse> {
        let expires_at = Instant::now() + Duration::from_secs(authorization.expires_in);
        let mut interval = authorization.interval.max(1);
        let scope_list = scopes.join(" ");

        loop {
            if Instant::now() >= expires_at {
                bail!("Twitch device authorization expired. Please run login again.");
            }

            sleep(Duration::from_secs(interval)).await;

            let form = Form::new()
                .text("client_id", client_id.to_owned())
                .text("scopes", scope_list.clone())
                .text("device_code", authorization.device_code.clone())
                .text("grant_type", DEVICE_GRANT);

            let response = self
                .http
                .post(TOKEN_ENDPOINT)
                .multipart(form)
                .send()
                .await
                .context("could not poll Twitch for device authorization")?;

            if response.status().is_success() {
                return response
                    .json::<TokenResponse>()
                    .await
                    .context("Twitch returned an invalid token response");
            }

            let status = response.status();
            let reason = match response.json::<TwitchError>().await {
                Ok(body) => body.message.or(body.error).map_or_else(
                    || String::from("unknown"),
                    |message| message,
                ),
                Err(_) => String::from("unknown"),
            };

            match (status, reason.as_str()) {
                (StatusCode::BAD_REQUEST, "authorization_pending") => continue,
                (StatusCode::BAD_REQUEST, "slow_down") => {
                    interval = interval.saturating_add(5);
                }
                (StatusCode::BAD_REQUEST, "access_denied") => {
                    bail!("Twitch authorization was denied.");
                }
                (StatusCode::BAD_REQUEST, "expired_token") => {
                    bail!("Twitch device authorization expired. Please run login again.");
                }
                _ => {
                    bail!("Twitch token request failed (HTTP {status}).");
                }
            }
        }
    }

    pub async fn validate_token(&self, access_token: &str) -> Result<Option<ValidatedUser>> {
        let response = self
            .http
            .get(VALIDATE_ENDPOINT)
            .header("Authorization", format!("OAuth {access_token}"))
            .send()
            .await
            .context("could not validate the Twitch access token")?;

        if response.status() == StatusCode::UNAUTHORIZED {
            return Ok(None);
        }

        if !response.status().is_success() {
            bail!(
                "Twitch token validation failed (HTTP {}).",
                response.status()
            );
        }

        response
            .json::<ValidatedUser>()
            .await
            .map(Some)
            .context("Twitch returned an invalid token validation response")
    }

    pub async fn refresh_token(
        &self,
        client_id: &str,
        refresh_token: &str,
    ) -> Result<TokenResponse> {
        let response = self
            .http
            .post(TOKEN_ENDPOINT)
            .form(&[
                ("client_id", client_id),
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
            ])
            .send()
            .await
            .context("could not refresh the Twitch access token")?;

        if !response.status().is_success() {
            bail!(
                "Twitch rejected the refresh token (HTTP {}). Run login again if the authorization has expired.",
                response.status()
            );
        }

        response
            .json::<TokenResponse>()
            .await
            .context("Twitch returned an invalid refreshed token response")
    }
}
