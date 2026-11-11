use anyhow::{Context, Result};
use keyring::{Entry, Error as KeyringError};
use serde::{Deserialize, Serialize};
use tokio::task;

const SERVICE_NAME: &str = "oxide";

#[derive(Serialize, Deserialize)]
pub struct StoredTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub scopes: Vec<String>,
    pub token_type: String,
}

pub struct TokenStore;

impl TokenStore {
    pub async fn load(client_id: &str) -> Result<Option<StoredTokens>> {
        let account = account_name(client_id);
        let raw = task::spawn_blocking(move || {
            let entry = Entry::new(SERVICE_NAME, &account)
                .context("could not access the operating system credential store")?;

            match entry.get_password() {
                Ok(value) => Ok(Some(value)),
                Err(KeyringError::NoEntry) => Ok(None),
                Err(error) => Err(anyhow::Error::new(error).context(
                    "could not read credentials from the operating system credential store",
                )),
            }
        })
        .await
        .context("credential-store task failed")??;

        raw.map(|value| {
            serde_json::from_str::<StoredTokens>(&value)
                .context("stored Twitch credentials are invalid; please run login again")
        })
        .transpose()
    }

    pub async fn save(client_id: &str, tokens: &StoredTokens) -> Result<()> {
        let account = account_name(client_id);
        let serialized =
            serde_json::to_string(tokens).context("could not serialize Twitch credentials")?;

        task::spawn_blocking(move || {
            let entry = Entry::new(SERVICE_NAME, &account)
                .context("could not access the operating system credential store")?;
            entry
                .set_password(&serialized)
                .context("could not save credentials to the operating system credential store")
        })
        .await
        .context("credential-store task failed")?
    }

    pub async fn delete(client_id: &str) -> Result<bool> {
        let account = account_name(client_id);

        task::spawn_blocking(move || {
            let entry = Entry::new(SERVICE_NAME, &account)
                .context("could not access the operating system credential store")?;

            match entry.delete_credential() {
                Ok(()) => Ok(true),
                Err(KeyringError::NoEntry) => Ok(false),
                Err(error) => Err(anyhow::Error::new(error).context(
                    "could not remove credentials from the operating system credential store",
                )),
            }
        })
        .await
        .context("credential-store task failed")?
    }
}

fn account_name(client_id: &str) -> String {
    format!("twitch:{client_id}")
}
