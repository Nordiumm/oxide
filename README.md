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

The login requests only these scopes:

- `user:read:chat` - read chat messages.
- `user:write:chat` - send chat messages.
- `moderator:manage:banned_users` - ban and unban users where the authenticated account has permission.

Access and refresh tokens are stored in the operating system credential store. Oxide does not write them to a plaintext configuration file or print them to the terminal. Public-client refresh tokens are single-use, so Oxide saves each replacement token immediately after a successful refresh.

## Configuration

General application configuration will be added in a later milestone. Never commit OAuth tokens, client secrets, or personal configuration files.

## Safety

Moderation features are planned to default to dry-run. Oxide must not take moderation actions unless explicitly enabled for a channel. Automated moderation should remain conservative and auditable.

## Contributing

Contributions, bug reports, and feature suggestions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

## License

Oxide is licensed under the [MIT License](LICENSE).
