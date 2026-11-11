//! Oxide - a tiny, local-first Twitch chat bot.

mod auth;
mod config;
mod eventsub;
pub mod helix;

use anyhow::{Context, Result, bail};
use auth::{AuthClient, REQUIRED_SCOPES, StoredTokens, TokenStore};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "oxide", version, about = "A local-first Twitch chat bot")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage Twitch authentication.
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
    /// Connect to Twitch chat.
    Chat {
        #[command(subcommand)]
        action: ChatAction,
    },
}

#[derive(Subcommand)]
enum ChatAction {
    /// Listen for messages in your Twitch channel.
    Listen {
        /// Twitch application client ID. Can also be set with OXIDE_CLIENT_ID.
        #[arg(long, env = "OXIDE_CLIENT_ID")]
        client_id: String,
    },
    /// Send a message to a Twitch channel as your authenticated account.
    Send {
        /// Twitch application client ID. Can also be set with OXIDE_CLIENT_ID.
        #[arg(long, env = "OXIDE_CLIENT_ID")]
        client_id: String,
        /// Destination channel login name, for example: somechannel.
        #[arg(long)]
        channel: String,
        /// Message to send. Put the message in quotes if it contains spaces.
        message: String,
    },
}

#[derive(Subcommand)]
enum AuthAction {
    /// Authenticate with Twitch using a device code.
    Login {
        /// Twitch application client ID. Can also be set with OXIDE_CLIENT_ID.
        #[arg(long, env = "OXIDE_CLIENT_ID")]
        client_id: String,
    },
    /// Check the saved Twitch authentication.
    Status {
        /// Twitch application client ID. Can also be set with OXIDE_CLIENT_ID.
        #[arg(long, env = "OXIDE_CLIENT_ID")]
        client_id: String,
    },
    /// Remove the saved Twitch authentication.
    Logout {
        /// Twitch application client ID. Can also be set with OXIDE_CLIENT_ID.
        #[arg(long, env = "OXIDE_CLIENT_ID")]
        client_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Auth { action } => handle_auth(action).await,
        Command::Chat { action } => handle_chat(action).await,
    }
}

async fn handle_auth(action: AuthAction) -> Result<()> {
    match action {
        AuthAction::Login { client_id } => login(&client_id).await,
        AuthAction::Status { client_id } => status(&client_id).await,
        AuthAction::Logout { client_id } => logout(&client_id).await,
    }
}

async fn handle_chat(action: ChatAction) -> Result<()> {
    match action {
        ChatAction::Listen { client_id } => listen(&client_id).await,
        ChatAction::Send {
            client_id,
            channel,
            message,
        } => send(&client_id, &channel, &message).await,
    }
}

async fn load_validated_tokens(client_id: &str) -> Result<(StoredTokens, auth::ValidatedUser)> {
    let mut tokens = TokenStore::load(client_id)
        .await?
        .context("No Twitch credentials found. Run: oxide auth login")?;

    let auth = AuthClient::new()?;
    let mut user = auth.validate_token(&tokens.access_token).await?;

    if user.is_none() {
        println!("The access token expired. Refreshing it...");
        let refreshed = auth.refresh_token(client_id, &tokens.refresh_token).await?;

        tokens = StoredTokens {
            access_token: refreshed.access_token,
            refresh_token: refreshed.refresh_token,
            scopes: if refreshed.scope.is_empty() {
                tokens.scopes
            } else {
                refreshed.scope
            },
            token_type: refreshed.token_type,
        };
        TokenStore::save(client_id, &tokens).await?;

        user = auth.validate_token(&tokens.access_token).await?;
    }

    let user = user.context("Twitch authentication is invalid. Run login again.")?;
    validate_identity(&user.client_id, client_id)?;
    validate_scopes(&user.scopes)?;

    Ok((tokens, user))
}

async fn listen(client_id: &str) -> Result<()> {
    let (tokens, user) = load_validated_tokens(client_id).await?;
    eventsub::listen(client_id, &tokens.access_token, &user.user_id).await
}

async fn send(client_id: &str, channel: &str, message: &str) -> Result<()> {
    let (tokens, user) = load_validated_tokens(client_id).await?;
    let broadcaster_id = helix::resolve_user_id(client_id, &tokens.access_token, channel).await?;

    let result = helix::send_message(
        client_id,
        &tokens.access_token,
        &broadcaster_id,
        &user.user_id,
        message,
    )
    .await?;

    if result.is_sent {
        println!("Message sent to {channel}.");
    } else if let Some(reason) = result.drop_reason {
        println!("Twitch did not send the message: {}", reason.message);
    } else {
        println!("Twitch did not send the message.");
    }

    Ok(())
}

async fn login(client_id: &str) -> Result<()> {
    let auth = AuthClient::new()?;
    let authorization = auth
        .begin_device_authorization(client_id, REQUIRED_SCOPES)
        .await?;

    println!("Open this page in your browser:");
    println!("{}", authorization.verification_uri);
    println!();
    println!("Enter this code: {}", authorization.user_code);
    println!("Waiting for Twitch authorization...");

    let response = auth
        .wait_for_device_token(client_id, REQUIRED_SCOPES, &authorization)
        .await?;

    let user = auth
        .validate_token(&response.access_token)
        .await?
        .context("Twitch issued a token that could not be validated")?;

    validate_identity(&user.client_id, client_id)?;
    validate_scopes(&user.scopes)?;

    let tokens = StoredTokens {
        access_token: response.access_token,
        refresh_token: response.refresh_token,
        scopes: response.scope,
        token_type: response.token_type,
    };

    TokenStore::save(client_id, &tokens).await?;
    println!("Authenticated as {}.", user.login);
    println!("Twitch account ID: {}", user.user_id);
    println!("Access token lifetime: {} seconds.", response.expires_in);
    println!("Credentials saved in the operating system credential store.");

    Ok(())
}

async fn status(client_id: &str) -> Result<()> {
    let Some(mut tokens) = TokenStore::load(client_id).await? else {
        println!("No Twitch account is connected. Run: oxide auth login");
        return Ok(());
    };

    let auth = AuthClient::new()?;
    let mut user = auth.validate_token(&tokens.access_token).await?;

    if user.is_none() {
        println!("The access token expired. Refreshing it...");
        let refreshed = auth.refresh_token(client_id, &tokens.refresh_token).await?;
        println!(
            "Twitch issued a new access token valid for {} seconds.",
            refreshed.expires_in
        );

        // Twitch public-client refresh tokens are single-use. Persist the
        // replacement immediately so a later validation/network error cannot
        // leave only the now-invalid previous refresh token on disk.
        tokens = StoredTokens {
            access_token: refreshed.access_token,
            refresh_token: refreshed.refresh_token,
            scopes: if refreshed.scope.is_empty() {
                tokens.scopes
            } else {
                refreshed.scope
            },
            token_type: refreshed.token_type,
        };
        TokenStore::save(client_id, &tokens).await?;

        let validated = auth
            .validate_token(&tokens.access_token)
            .await?
            .context("Twitch returned a refreshed token that could not be validated")?;

        validate_identity(&validated.client_id, client_id)?;
        validate_scopes(&validated.scopes)?;

        user = Some(validated);
        println!("Access token refreshed and new credentials saved.");
    }

    let user = user.context("Twitch authentication is invalid. Run login again.")?;
    validate_identity(&user.client_id, client_id)?;
    validate_scopes(&user.scopes)?;
    println!("Connected to Twitch as {}.", user.login);
    println!("Twitch account ID: {}", user.user_id);
    println!(
        "Access token validity reported by Twitch: {} seconds.",
        user.expires_in
    );
    println!("Granted scopes: {}", user.scopes.join(", "));

    Ok(())
}

async fn logout(client_id: &str) -> Result<()> {
    if TokenStore::delete(client_id).await? {
        println!("Twitch credentials removed from the operating system credential store.");
    } else {
        println!("No saved Twitch credentials were found.");
    }

    Ok(())
}

fn validate_identity(actual_client_id: &str, expected_client_id: &str) -> Result<()> {
    if actual_client_id != expected_client_id {
        bail!("The saved Twitch credentials belong to a different application client ID.");
    }

    Ok(())
}

fn validate_scopes(granted_scopes: &[String]) -> Result<()> {
    let missing: Vec<&str> = REQUIRED_SCOPES
        .iter()
        .copied()
        .filter(|required| {
            !granted_scopes
                .iter()
                .any(|granted| granted.as_str() == *required)
        })
        .collect();

    if !missing.is_empty() {
        bail!(
            "Twitch authorization is missing required scopes: {}. Run login again and approve all requested permissions.",
            missing.join(", ")
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_identity, validate_scopes};
    use crate::auth::REQUIRED_SCOPES;

    #[test]
    fn accepts_matching_client_id() {
        assert!(validate_identity("client-123", "client-123").is_ok());
    }

    #[test]
    fn rejects_different_client_id() {
        assert!(validate_identity("client-123", "client-456").is_err());
    }

    #[test]
    fn accepts_all_required_scopes() {
        let scopes = REQUIRED_SCOPES
            .iter()
            .map(|scope| (*scope).to_owned())
            .collect::<Vec<_>>();

        assert!(validate_scopes(&scopes).is_ok());
    }

    #[test]
    fn rejects_missing_scopes() {
        assert!(validate_scopes(&["user:read:chat".to_owned()]).is_err());
    }
}
