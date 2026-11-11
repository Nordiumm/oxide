//! Local Oxide configuration.

use std::{collections::BTreeMap, fs};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Deserialize)]
pub struct Config {
    pub channels: BTreeMap<String, bool>,
}

impl Config {
    /// Loads channel configuration from config.toml in the current directory.
    pub fn load() -> Result<Self> {
        let contents = fs::read_to_string(CONFIG_FILE).with_context(|| {
            format!(
                "could not read {CONFIG_FILE}; create it using config.example.toml as a template"
            )
        })?;

        let config: Self =
            toml::from_str(&contents).context("config.toml contains invalid TOML")?;

        if config.channels.keys().any(|name| name.trim().is_empty()) {
            bail!("channel names in config.toml cannot be empty");
        }

        Ok(config)
    }

    /// Returns enabled channel logins in deterministic alphabetical order.
    pub fn enabled_channels(&self) -> impl Iterator<Item = &str> {
        self.channels
            .iter()
            .filter(|(_, enabled)| **enabled)
            .map(|(login, _)| login.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn loads_enabled_and_disabled_channels() {
        let config: Config = toml::from_str(
            "[channels]\nnordiumm = true\nripo5015 = false\n",
        )
        .expect("configuration should parse");

        assert_eq!(
            config.enabled_channels().collect::<Vec<_>>(),
            vec!["nordiumm"]
        );
    }

    #[test]
    fn accepts_an_empty_channel_table() {
        let config: Config =
            toml::from_str("[channels]\n").expect("empty channel table should parse");
        assert_eq!(config.enabled_channels().count(), 0);
    }
}
