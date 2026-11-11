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

The listener connects to Twitch EventSub over WebSocket and subscribes to `channel.chat.message`. Incoming chat messages are printed to the terminal. Press Ctrl+C to stop it.

Oxide reads enabled channels from `config.toml`. It checks the authenticated account's moderated-channel list before subscribing. Your own channel is also eligible. Channels where your account is not a moderator are skipped completely: Oxide will not listen, respond to commands, send timers, or moderate there.

Create `config.toml` in the repository root using `config.example.toml` as a template:

```toml
[channels]
nordiumm = true
ripo5015 = true
```

Set a channel to `false` to disable it. The listener currently checks eligibility when it starts, so restart Oxide after changing moderator status or the configuration.

The login requests only these scopes:

- `user:read:chat` - read chat messages.
- `user:write:chat` - send chat messages.
- `moderator:manage:banned_users` - ban and unban users where the authenticated account has permission.
- `user:read:moderated_channels` - check which channels the authenticated account moderates.

Access and refresh tokens are stored in the operating system credential store. Oxide does not write them to a plaintext configuration file or print them to the terminal. Public-client refresh tokens are single-use, so Oxide saves each replacement token immediately after a successful refresh.

## Configuration

Copy `config.example.toml` to `config.toml` and edit the `[channels]` table. The local `config.toml` is ignored by Git. Never commit OAuth tokens, client secrets, or personal configuration files.

## Safety

Moderation features are planned to default to dry-run. Oxide must not take moderation actions unless explicitly enabled for a channel. Automated moderation should remain conservative and auditable.

## Contributing

Contributions, bug reports, and feature suggestions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

## License

Oxide is licensed under the [MIT License](LICENSE).
