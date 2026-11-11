# Oxide

A tiny, fast, local-first Twitch chat bot written in Rust. No servers, no accounts to host.

> **Status:** Early development - v0.1 is not ready for production use.

Oxide is designed to run as a single binary on your own computer. It uses Twitch's official APIs and keeps its configuration, credentials, and action logs locally.

## Goals

- Run locally on Windows, macOS, and Linux.
- Connect to Twitch chat using EventSub over WebSocket.
- Authenticate using Twitch OAuth Device Code Grant.
- Respond to configurable chat commands and timed messages.
- Detect likely spam with conservative, explainable rules.
- Keep moderation in dry-run mode by default.
- Make the project approachable for contributors.

## Project status

Oxide is under active development. Features will be added incrementally; see the project milestones and future issues for planned work.

## Development

### Requirements

- Rust stable toolchain
- Cargo

### Build

```sh
cargo build --workspace
```

### Run

```sh
cargo run -p oxide -- --help
```

### Check

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Twitch authentication

Create a Twitch application and provide its client ID through the `OXIDE_CLIENT_ID` environment variable, or pass it with `--client-id`.

PowerShell:

```powershell
$env:OXIDE_CLIENT_ID = "your-client-id"
cargo run -p oxide -- auth login
```

Linux and macOS:

```sh
export OXIDE_CLIENT_ID="your-client-id"
cargo run -p oxide -- auth login
```

Oxide displays a Twitch activation URL and one-time code. Open the URL, enter the code, and approve the requested permissions.

Available commands:

```sh
cargo run -p oxide -- auth login
cargo run -p oxide -- auth status
cargo run -p oxide -- auth logout
```

Send a message to a channel as your authenticated Twitch account:

```powershell
cargo run -p oxide -- chat send --channel somechannel "Hello chat!"
```

Oxide resolves the channel login to its Twitch user ID, then sends the message through the official Helix API. Your authenticated account must be permitted to chat in the destination channel.

## Twitch chat

Listen for messages in configured Twitch channels where your authenticated account is the broadcaster or a moderator:

```powershell
cargo run -p oxide -- chat listen
```

The listener connects to Twitch EventSub over WebSocket and subscribes to `channel.chat.message`. Incoming chat messages are printed to the terminal. Configured commands are matched per channel and answered through your authenticated Twitch account. Press Ctrl+C to stop it.

Oxide reads enabled channels from `config.toml`. It checks the authenticated account's moderated-channel list before subscribing. Your own channel is also eligible. Channels where your account is not a moderator are skipped completely: Oxide will not listen, respond to commands, send timers, or moderate there.

Create `config.toml` in the repository root using `config.example.toml` as a template:

```toml
[channels]
nordiumm = true
ripo5015 = true
```

Set a channel to `false` to disable it. The listener currently checks eligibility when it starts, so restart Oxide after changing moderator status or the configuration.

The login requests these scopes:

- `user:read:chat` - read chat messages.
- `user:write:chat` - send chat messages.
- `moderator:manage:banned_users` - ban and unban users where the authenticated account has permission.
- `moderator:manage:chat_messages` - delete individual chat messages where the authenticated account has permission.
- `user:read:moderated_channels` - check which channels the authenticated account moderates.

Access and refresh tokens are stored in the operating system credential store. Oxide does not write them to a plaintext configuration file or print them to the terminal. Public-client refresh tokens are single-use, so Oxide saves each replacement token immediately after a successful refresh.

## Configuration

Copy `config.example.toml` to `config.toml` and edit the `[channels]` table. The local `config.toml` is ignored by Git. Never commit OAuth tokens, client secrets, or personal configuration files.

### Chat commands

Copy `commands.example.toml` to `commands.toml` to configure per-channel chat commands. The local `commands.toml` is ignored by Git.

```toml
[channels.ripo5015.commands.discord]
response = "Join my Discord: https://discord.gg/example"

[channels.ripo5015.commands.github]
response = "Check out my GitHub: https://github.com/Nordiumm"

[channels.nordiumm.commands.discord]
response = "Join my Discord: https://discord.gg/example"
```

A viewer typing `!discord` in `ripo5015` receives the response configured under that channel. Commands are matched without case sensitivity, and extra words after the command are ignored. Commands are isolated by channel. Missing `commands.toml` means no commands are active. Oxide ignores its own messages to prevent response loops.

### Automatic spam moderation

Copy `moderation.example.toml` to `moderation.toml` to configure per-channel spam phrases. The local file is ignored by Git.

Moderation is conservative phrase matching, not an AI classifier. It only matches phrases explicitly listed for an enabled channel. By default, `dry_run = true`, so Oxide logs what it would delete and ban without taking action. Review the matches first. Set `dry_run = false` only when the rules are correct.

Active moderation deletes the matching message and bans the account using Twitch's official Helix moderation endpoints. Your authenticated account must be the broadcaster or a moderator in that channel. Twitch may reject actions against protected users or messages.

The new `moderator:manage:chat_messages` permission is required in addition to `moderator:manage:banned_users`. After updating Oxide, run `auth login` again and approve the additional permission.

## Safety

Automated moderation defaults to dry-run and requires explicit channel rules. Matching phrases and moderation actions are logged for review.

## Contributing

Contributions, bug reports, and feature suggestions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

## License

Oxide is licensed under the [MIT License](LICENSE).
