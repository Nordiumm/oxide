//! Conservative, configurable spam moderation.

use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

const MODERATION_FILE: &str = "moderation.toml";

fn default_dry_run() -> bool {
    true
}

#[derive(Debug, Default, Deserialize)]
pub struct ModerationConfig {
    #[serde(default = "default_dry_run")]
    dry_run: bool,
    #[serde(default)]
    channels: BTreeMap<String, ChannelModeration>,
}

#[derive(Debug, Default, Deserialize)]
struct ChannelModeration {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    phrases: Vec<String>,
}

impl ModerationConfig {
    /// Loads moderation.toml. A missing file disables automated moderation.
    pub fn load() -> Result<Self> {
        if !Path::new(MODERATION_FILE).exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(MODERATION_FILE)
            .with_context(|| format!("could not read {MODERATION_FILE}"))?;
        let config: Self =
            toml::from_str(&contents).context("moderation.toml contains invalid TOML")?;

        for (channel, settings) in &config.channels {
            if channel.trim().is_empty() {
                bail!("channel names in moderation.toml cannot be empty");
            }
            if settings.enabled
                && !settings.phrases.iter().any(|phrase| !phrase.trim().is_empty())
            {
                bail!("enabled moderation for {channel} needs at least one spam phrase");
            }
        }

        Ok(config)
    }

    pub fn dry_run(&self) -> bool {
        self.dry_run
    }

    /// Returns the configured phrase that matched, if moderation is enabled for the channel.
    pub fn matching_phrase(&self, channel: &str, message: &str) -> Option<&str> {
        let settings = self
            .channels
            .iter()
            .find(|(login, _)| login.eq_ignore_ascii_case(channel))
            .map(|(_, settings)| settings)?;

        if !settings.enabled {
            return None;
        }

        let message = message.to_lowercase();
        settings
            .phrases
            .iter()
            .find(|phrase| {
                !phrase.trim().is_empty() && message.contains(&phrase.to_lowercase())
            })
            .map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::ModerationConfig;

    #[test]
    fn matches_configured_spam_phrase_case_insensitively() {
        let config: ModerationConfig = toml::from_str(
            r#"
dry_run = true

[channels.nordiumm]
enabled = true
phrases = ["buy twitch viewers"]
"#,
        )
        .expect("moderation config should parse");

        assert_eq!(
            config.matching_phrase("Nordiumm", "BUY TWITCH VIEWERS now"),
            Some("buy twitch viewers")
        );
    }

    #[test]
    fn does_not_match_disabled_channels() {
        let config: ModerationConfig = toml::from_str(
            r#"
[channels.nordiumm]
enabled = false
phrases = ["buy twitch viewers"]
"#,
        )
        .expect("moderation config should parse");

        assert_eq!(
            config.matching_phrase("nordiumm", "buy twitch viewers"),
            None
        );
    }

    #[test]
    fn dry_run_defaults_to_true() {
        let config = ModerationConfig::default();
        assert!(config.dry_run());
    }
}
