# zoom-cli

Agent-friendly CLI for the Zoom API with JSON output, structured exit codes, and schema introspection.

## Installation

```bash
# via cargo
cargo install zoom-cli

# via pip
pip install zoom-cli
```

## Configuration

Run `zoom init` to set up credentials interactively, or create `~/.config/zoom-cli/config.toml`:

```toml
[default]
account_id = "YOUR_ACCOUNT_ID"
client_id = "YOUR_CLIENT_ID"
client_secret = "YOUR_CLIENT_SECRET"
```

Requires a [Zoom Server-to-Server OAuth app](https://marketplace.zoom.us/develop/create).

## Usage

```
zoom meetings list
zoom meetings get <id>
zoom meetings create --topic "Standup" --duration 30
zoom recordings list --from 2026-01-01
zoom users me
zoom reports meetings --from 2026-01-01
zoom init
```

## License

MIT
