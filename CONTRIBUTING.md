# Contributing to Oxide

Thank you for your interest in contributing to Oxide!

Oxide is an open-source, local-first Twitch chat bot written in Rust. Contributions should keep the project safe by default, cross-platform, understandable, and easy to maintain.

## Before you start

- Check existing issues and pull requests to avoid duplicate work.
- For substantial changes, open an issue first to discuss the approach.
- Keep changes focused. Prefer small pull requests over large rewrites.

## Development setup

1. Install the stable Rust toolchain.
2. Fork the repository and clone your fork.
3. Create a branch for your change.
4. Make your changes and add or update tests.
5. Run the checks below before opening a pull request.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Pull requests

Please include:

- A clear explanation of the problem and solution.
- Tests for new behavior where practical.
- Documentation updates when user-facing behavior changes.
- Any relevant compatibility or safety considerations.

Keep pull requests focused and avoid unrelated formatting or refactoring.

## Security and credentials

- Never commit OAuth tokens, client secrets, or personal configuration.
- Do not include credentials in logs, screenshots, issues, or pull requests.
- Report suspected security vulnerabilities privately to the repository maintainer rather than publishing exploit details in an issue.

## Design principles

- Use Twitch's official APIs only.
- Keep secrets and runtime data on the user's machine.
- Default moderation to dry-run.
- Never moderate the broadcaster, moderators, VIPs, subscribers, or allowlisted users.
- Make automated decisions explainable and record moderation decisions locally.
- Preserve Windows, macOS, and Linux support.

By contributing, you agree that your contributions are provided under the project's MIT license.
