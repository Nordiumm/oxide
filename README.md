# Oxide

A tiny, fast, local-first Twitch chat bot written in Rust. No servers, no accounts to host.

> **Status:** Early development - v0.1 is not ready for production use.

Oxide is designed to run as a single binary on your own computer. It will use Twitch's official APIs and keep its configuration, credentials, and action logs locally.

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
cargo run -p oxide
```

### Check

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Configuration

Configuration documentation will be added when the configuration and validation milestone is implemented. Never commit OAuth tokens, client secrets, or personal configuration files.

## Safety

Moderation features are planned to default to dry-run. Oxide must not take moderation actions unless explicitly enabled for a channel. Automated moderation should remain conservative and auditable.

## Contributing

Contributions, bug reports, and feature suggestions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

## License

Oxide is licensed under the [MIT License](LICENSE).
