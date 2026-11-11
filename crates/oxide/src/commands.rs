//! Per-channel configurable chat commands.

use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

const COMMANDS_FILE: &str = "commands.toml";

#[derive(Debug, Default, Deserialize)]
pub struct CommandsConfig {
    #[serde(default)]
    channels: BTreeMap<String, ChannelCommands>,
}

#[derive(Debug, Default, Deserialize)]
struct ChannelCommands {
    #[serde(default)]
    commands: BTreeMap<String, CommandDefinition>,
}

#[derive(Debug, Deserialize)]
struct CommandDefinition {
    response: String,
}

impl CommandsConfig {
    /// Loads commands.toml. A missing file means no commands are configured.
    pub fn load() -> Result<Self> {
        if !Path::new(COMMANDS_FILE).exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(COMMANDS_FILE)
            .with_context(|| format!("could not read {COMMANDS_FILE}"))?;
        let config: Self =
            toml::from_str(&contents).context("commands.toml contains invalid TOML")?;

        for (channel, settings) in &config.channels {
            if channel.trim().is_empty() {
                bail!("channel names in commands.toml cannot be empty");
            }

            for (name, command) in &settings.commands {
                if name.trim().is_empty() || name.starts_with('!') || name.contains(char::is_whitespace) {
                    bail!("command names must be non-empty and must not include ! or whitespace");
                }
                if command.response.trim().is_empty() {
                    bail!("the response for !{name} in channel {channel} cannot be empty");
                }
            }
        }

        Ok(config)
    }

    /// Returns the configured response for a command in a specific channel.
    pub fn response_for(&self, channel: &str, message: &str) -> Option<&str> {
        let command_name = message
            .strip_prefix('!')?
            .split_whitespace()
            .next()?;

        self.channels
            .iter()
            .find(|(login, _)| login.eq_ignore_ascii_case(channel))
            .and_then(|(_, settings)| {
                settings
                    .commands
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case(command_name))
            })
            .map(|(_, command)| command.response.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::CommandsConfig;

    #[test]
    fn finds_a_command_in_its_channel() {
        let config: CommandsConfig = toml::from_str(
            r#"[channels.ripo5015.commands.discord]
response = "Join my Discord"
"#,
        )
        .expect("commands should parse");

        assert_eq!(
            config.response_for("RIPo5015", "!discord"),
            Some("Join my Discord")
        );
        assert_eq!(config.response_for("nordiumm", "!discord"), None);
    }

    #[test]
    fn command_names_are_case_insensitive_and_allow_arguments() {
        let config: CommandsConfig = toml::from_str(
            r#"[channels.ripo5015.commands.discord]
response = "Join my Discord"
"#,
        )
        .expect("commands should parse");

        assert_eq!(
            config.response_for("ripo5015", "!DISCORD please"),
            Some("Join my Discord")
        );
    }

    #[test]
    fn ignores_messages_that_are_not_commands() {
        let config = CommandsConfig::default();
        assert_eq!(config.response_for("ripo5015", "hello"), None);
    }
}
