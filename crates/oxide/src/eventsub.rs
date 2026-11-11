//! Twitch EventSub WebSocket connection and chat notifications.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde_json::{Value, json};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{commands::CommandsConfig, helix};

const EVENTSUB_URL: &str = "wss://eventsub.wss.twitch.tv/ws";
const SUBSCRIPTIONS_URL: &str = "https://api.twitch.tv/helix/eventsub/subscriptions";

/// A chat message received from one of the subscribed channels.
struct ChatMessage {
    channel: String,
    broadcaster_id: String,
    chatter_id: String,
    username: String,
    text: String,
}

/// Connects to Twitch EventSub and handles messages from eligible configured channels.
pub async fn listen(
    client_id: &str,
    access_token: &str,
    user_id: &str,
    user_login: &str,
    channels: &[(String, String)],
    commands: &CommandsConfig,
) -> Result<()> {
    if channels.is_empty() {
        bail!("No enabled channels are eligible. Check config.toml and your moderator permissions.");
    }
    println!("Connecting to Twitch EventSub...");

    let (socket, _) = connect_async(EVENTSUB_URL)
        .await
        .context("could not connect to Twitch EventSub")?;
    let (mut sender, mut receiver) = socket.split();

    let welcome = timeout(Duration::from_secs(15), async {
        loop {
            match receiver.next().await {
                Some(Ok(Message::Text(text))) => {
                    let value: Value = serde_json::from_str(text.as_str())
                        .context("Twitch sent an invalid EventSub message")?;
                    if value
                        .pointer("/metadata/message_type")
                        .and_then(Value::as_str)
                        == Some("session_welcome")
                    {
                        break Ok::<Value, anyhow::Error>(value);
                    }
                }
                Some(Ok(Message::Ping(payload))) => {
                    sender
                        .send(Message::Pong(payload))
                        .await
                        .context("could not respond to Twitch WebSocket ping")?;
                }
                Some(Ok(Message::Close(_))) | None => {
                    bail!("Twitch closed the EventSub connection before welcoming Oxide");
                }
                Some(Ok(_)) => {}
                Some(Err(error)) => {
                    return Err(anyhow::Error::new(error).context("EventSub WebSocket error"));
                }
            }
        }
    })
    .await
    .context("timed out waiting for the EventSub welcome message")??;

    let session_id = welcome
        .pointer("/payload/session/id")
        .and_then(Value::as_str)
        .context("EventSub welcome message did not include a session ID")?;

    let keepalive_timeout = welcome
        .pointer("/payload/session/keepalive_timeout_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(10);

    for (login, broadcaster_id) in channels {
        subscribe_to_chat(
            client_id,
            access_token,
            user_id,
            broadcaster_id,
            login,
            session_id,
        )
        .await?;
    }

    println!(
        "Connected to {} Twitch channel(s). Press Ctrl+C to stop.",
        channels.len()
    );
    println!("Messages from eligible channels will appear below.");

    loop {
        let next_message = timeout(
            Duration::from_secs(keepalive_timeout.saturating_add(5)),
            receiver.next(),
        );

        tokio::select! {
            signal = tokio::signal::ctrl_c() => {
                signal.context("could not listen for Ctrl+C")?;
                println!("Stopping Oxide chat listener.");
                break;
            }
            incoming = next_message => {
                match incoming {
                    Err(_) => bail!("EventSub timed out waiting for a notification or keepalive"),
                    Ok(None) => bail!("Twitch closed the EventSub connection"),
                    Ok(Some(Err(error))) => {
                        return Err(anyhow::Error::new(error).context("EventSub WebSocket error"));
                    }
                    Ok(Some(Ok(Message::Text(text)))) => {
                        if let Some(chat_message) = handle_eventsub_message(text.as_str())? {
                            println!(
                                "[{}] {}: {}",
                                chat_message.channel, chat_message.username, chat_message.text
                            );

                            // Never respond to messages sent by Oxide's own Twitch account.
                            if !chat_message.chatter_id.is_empty()
                                && chat_message.chatter_id != user_id
                            {
                                if let Some(response) =
                                    commands.response_for(&chat_message.channel, &chat_message.text)
                                {
                                    let result = helix::send_message(
                                        client_id,
                                        access_token,
                                        &chat_message.broadcaster_id,
                                        user_id,
                                        response,
                                    )
                                    .await?;

                                    if result.is_sent {
                                        println!(
                                            "[{}] Oxide: {}",
                                            chat_message.channel, response
                                        );
                                    } else if let Some(reason) = result.drop_reason {
                                        eprintln!(
                                            "Twitch did not send Oxide's response in {}: {}",
                                            chat_message.channel, reason.message
                                        );
                                    }
                                }
                            } else if chat_message.username.eq_ignore_ascii_case(user_login) {
                                // The account login is a fallback if Twitch omits the chatter ID.
                            }
                        }
                    }
                    Ok(Some(Ok(Message::Ping(payload)))) => {
                        sender.send(Message::Pong(payload)).await
                            .context("could not respond to Twitch WebSocket ping")?;
                    }
                    Ok(Some(Ok(Message::Close(_)))) => {
                        bail!("Twitch closed the EventSub connection");
                    }
                    Ok(Some(Ok(_))) => {}
                }
            }
        }
    }

    Ok(())
}

async fn subscribe_to_chat(
    client_id: &str,
    access_token: &str,
    user_id: &str,
    broadcaster_id: &str,
    channel_login: &str,
    session_id: &str,
) -> Result<()> {
    let http = Client::new();
    let response = http
        .post(SUBSCRIPTIONS_URL)
        .header("Client-Id", client_id)
        .bearer_auth(access_token)
        .json(&json!({
            "type": "channel.chat.message",
            "version": "1",
            "condition": {
                "broadcaster_user_id": broadcaster_id,
                "user_id": user_id
            },
            "transport": {
                "method": "websocket",
                "session_id": session_id
            }
        }))
        .send()
        .await
        .context("could not create the Twitch chat EventSub subscription")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("Twitch rejected the chat subscription (HTTP {status}): {body}");
    }

    println!("Subscribed to chat messages for {channel_login}.");
    Ok(())
}

/// Parses EventSub messages and returns chat notifications, if present.
fn handle_eventsub_message(raw: &str) -> Result<Option<ChatMessage>> {
    let message: Value =
        serde_json::from_str(raw).context("Twitch sent an invalid EventSub message")?;

    match message
        .pointer("/metadata/message_type")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
    {
        "session_keepalive" => Ok(None),
        "notification" => {
            if message
                .pointer("/metadata/subscription_type")
                .and_then(Value::as_str)
                != Some("channel.chat.message")
            {
                return Ok(None);
            }

            let event = message
                .pointer("/payload/event")
                .context("chat notification did not contain an event")?;

            Ok(Some(ChatMessage {
                channel: event
                    .get("broadcaster_user_name")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown channel")
                    .to_owned(),
                broadcaster_id: event
                    .get("broadcaster_user_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                chatter_id: event
                    .get("chatter_user_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                username: event
                    .get("chatter_user_name")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_owned(),
                text: event
                    .pointer("/message/text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
            }))
        }
        "session_reconnect" => {
            bail!(
                "Twitch requested an EventSub reconnect. Restart Oxide with oxide chat listen for now."
            );
        }
        "revocation" => {
            bail!(
                "Twitch revoked the EventSub subscription. Check your Twitch authorization and run login again if needed."
            );
        }
        other => {
            eprintln!("Received an unhandled EventSub message: {other}");
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::handle_eventsub_message;

    #[test]
    fn parses_chat_notifications() {
        let message = r#"{
            "metadata": {
                "message_type": "notification",
                "subscription_type": "channel.chat.message"
            },
            "payload": {
                "event": {
                    "broadcaster_user_id": "channel-123",
                    "broadcaster_user_name": "ripo5015",
                    "chatter_user_id": "viewer-456",
                    "chatter_user_name": "viewer",
                    "message": { "text": "!discord" }
                }
            }
        }"#;

        let parsed = handle_eventsub_message(message)
            .expect("notification should parse")
            .expect("chat notification should be returned");
        assert_eq!(parsed.channel, "ripo5015");
        assert_eq!(parsed.broadcaster_id, "channel-123");
        assert_eq!(parsed.chatter_id, "viewer-456");
        assert_eq!(parsed.text, "!discord");
    }

    #[test]
    fn accepts_keepalive_messages() {
        let message = r#"{"metadata":{"message_type":"session_keepalive"},"payload":{}}"#;
        assert!(handle_eventsub_message(message).expect("keepalive should parse").is_none());
    }
}
