//! Twitch Helix chat message sending and user lookup.

use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const SEND_MESSAGE_URL: &str = "https://api.twitch.tv/helix/chat/messages";
const USERS_URL: &str = "https://api.twitch.tv/helix/users";
const MODERATED_CHANNELS_URL: &str = "https://api.twitch.tv/helix/moderation/channels";
const MAX_MESSAGE_CHARACTERS: usize = 500;

/// Result returned by Twitch after attempting to send a chat message.
#[derive(Debug, Deserialize)]
pub struct SendMessageResult {
    /// Whether Twitch accepted the message for delivery.
    pub is_sent: bool,
    /// Twitch's message ID when the message was sent.
    pub message_id: Option<String>,
    /// Reason Twitch did not send the message, if applicable.
    pub drop_reason: Option<DropReason>,
}

/// Explains why Twitch did not send a chat message.
#[derive(Debug, Deserialize)]
pub struct DropReason {
    /// Machine-readable reason code.
    pub code: String,
    /// Human-readable explanation.
    pub message: String,
}

#[derive(Serialize)]
struct SendMessageRequest<'a> {
    broadcaster_id: &'a str,
    sender_id: &'a str,
    message: &'a str,
}

#[derive(Deserialize)]
struct UsersResponse {
    data: Vec<TwitchUser>,
}

#[derive(Deserialize)]
struct TwitchUser {
    id: String,
}

#[derive(Deserialize)]
struct ModeratedChannelsResponse {
    data: Vec<ModeratedChannel>,
    pagination: Pagination,
}

#[derive(Deserialize)]
struct ModeratedChannel {
    broadcaster_id: String,
}

#[derive(Default, Deserialize)]
struct Pagination {
    cursor: Option<String>,
}

/// Returns every channel where the authenticated user currently has moderator privileges.
pub async fn moderated_channel_ids(
    client_id: &str,
    access_token: &str,
    user_id: &str,
) -> Result<HashSet<String>> {
    let http = build_http_client()?;
    let mut channels = HashSet::new();
    let mut cursor: Option<String> = None;

    loop {
        let mut request = http
            .get(MODERATED_CHANNELS_URL)
            .header("Client-Id", client_id)
            .bearer_auth(access_token)
            .query(&[("user_id", user_id), ("first", "100")]);

        if let Some(value) = cursor.as_deref() {
            request = request.query(&[("after", value)]);
        }

        let response = request
            .send()
            .await
            .context("could not retrieve channels moderated by the authenticated account")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            bail!("Twitch rejected the moderated-channel lookup (HTTP {status}): {body}");
        }

        let page = response
            .json::<ModeratedChannelsResponse>()
            .await
            .context("Twitch returned an invalid moderated-channel response")?;

        channels.extend(page.data.into_iter().map(|channel| channel.broadcaster_id));
        cursor = page.pagination.cursor;

        if cursor.is_none() {
            break;
        }
    }

    Ok(channels)
}

/// Looks up a Twitch account by login name and returns its user ID.
pub async fn resolve_user_id(client_id: &str, access_token: &str, login: &str) -> Result<String> {
    let login = login.trim().trim_start_matches('@');
    if login.is_empty() {
        bail!("The Twitch channel name cannot be empty.");
    }

    let http = build_http_client()?;
    let response = http
        .get(USERS_URL)
        .header("Client-Id", client_id)
        .bearer_auth(access_token)
        .query(&[("login", login)])
        .send()
        .await
        .context("could not look up the Twitch channel")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("Twitch rejected the channel lookup (HTTP {status}): {body}");
    }

    let response = response
        .json::<UsersResponse>()
        .await
        .context("Twitch returned an invalid user lookup response")?;

    response
        .data
        .into_iter()
        .next()
        .map(|user| user.id)
        .with_context(|| format!("Twitch channel '{login}' was not found"))
}

/// Sends a message as the authenticated user to the specified broadcaster's chat.
///
/// The sender ID must belong to the user associated with the access token.
/// The broadcaster ID identifies the destination channel and may be different
/// from the sender's user ID. Twitch still decides whether the account is
/// permitted to send messages in the destination channel.
pub async fn send_message(
    client_id: &str,
    access_token: &str,
    broadcaster_id: &str,
    sender_id: &str,
    message: &str,
) -> Result<SendMessageResult> {
    validate_message(message)?;

    if broadcaster_id.trim().is_empty() {
        bail!("The target broadcaster ID cannot be empty.");
    }

    if sender_id.trim().is_empty() {
        bail!("The sender user ID cannot be empty.");
    }

    let http = build_http_client()?;
    let response = http
        .post(SEND_MESSAGE_URL)
        .header("Client-Id", client_id)
        .bearer_auth(access_token)
        .json(&SendMessageRequest {
            broadcaster_id,
            sender_id,
            message,
        })
        .send()
        .await
        .context("could not send the Twitch chat message")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("Twitch rejected the chat message (HTTP {status}): {body}");
    }

    let response = response
        .json::<SendMessageResponse>()
        .await
        .context("Twitch returned an invalid chat message response")?;

    response
        .data
        .into_iter()
        .next()
        .context("Twitch returned an empty chat message response")
}

fn build_http_client() -> Result<Client> {
    Client::builder()
        .user_agent(concat!("oxide/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("could not initialize the Twitch HTTP client")
}

/// Rejects empty messages and messages longer than Twitch's character limit.
fn validate_message(message: &str) -> Result<()> {
    if message.trim().is_empty() {
        bail!("A chat message cannot be empty.");
    }

    let character_count = message.chars().count();
    if character_count > MAX_MESSAGE_CHARACTERS {
        bail!(
            "A Twitch chat message cannot exceed {MAX_MESSAGE_CHARACTERS} characters (received {character_count})."
        );
    }

    Ok(())
}

#[derive(Deserialize)]
struct SendMessageResponse {
    data: Vec<SendMessageResult>,
}

#[cfg(test)]
mod tests {
    use super::{DropReason, SendMessageResponse, validate_message};

    #[test]
    fn accepts_a_normal_chat_message() {
        assert!(validate_message("Hello chat!").is_ok());
    }

    #[test]
    fn rejects_empty_chat_messages() {
        assert!(validate_message("").is_err());
        assert!(validate_message("   ").is_err());
    }

    #[test]
    fn accepts_a_message_at_the_character_limit() {
        assert!(validate_message(&"a".repeat(500)).is_ok());
    }

    #[test]
    fn rejects_a_message_over_the_character_limit() {
        assert!(validate_message(&"a".repeat(501)).is_err());
    }

    #[test]
    fn parses_a_successful_twitch_response() {
        let response =
            r#"{"data":[{"message_id":"message-123","is_sent":true,"drop_reason":null}]}"#;
        let parsed: SendMessageResponse =
            serde_json::from_str(response).expect("response should be valid");
        assert_eq!(parsed.data.len(), 1);
        assert!(parsed.data[0].is_sent);
        assert_eq!(parsed.data[0].message_id.as_deref(), Some("message-123"));
        assert!(parsed.data[0].drop_reason.is_none());
    }

    #[test]
    fn parses_a_twitch_drop_reason() {
        let reason = r#"{"code":"automod_held","message":"Your message was held for review."}"#;
        let parsed: DropReason = serde_json::from_str(reason).expect("drop reason should be valid");
        assert_eq!(parsed.code, "automod_held");
        assert_eq!(parsed.message, "Your message was held for review.");
    }
}
