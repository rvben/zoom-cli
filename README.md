[![CI](https://github.com/rvben/zoom-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/rvben/zoom-cli/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/zoom-cli.svg)](https://crates.io/crates/zoom-cli)
[![PyPI](https://img.shields.io/pypi/v/zoom-cli.svg)](https://pypi.org/project/zoom-cli/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![codecov](https://codecov.io/gh/rvben/zoom-cli/graph/badge.svg)](https://codecov.io/gh/rvben/zoom-cli)

# zoom-cli

A command-line interface for the [Zoom API](https://developers.zoom.us/docs/api/) -- manage meetings, recordings, users, webinars, and reports from your terminal.

## Install

```bash
# From crates.io
cargo install zoom-cli

# From PyPI (pre-built binaries, no Rust toolchain needed)
pip install zoom-cli

# From GitHub Releases (Linux, macOS)
curl -fsSL https://github.com/rvben/zoom-cli/releases/latest/download/zoom-$(uname -m)-unknown-linux-gnu.tar.gz | tar xz
```

## Quick Start

```bash
# Interactive setup (creates config with your Zoom OAuth credentials)
zoom init

# List your upcoming meetings
zoom meetings list

# Create a meeting
zoom meetings create --topic "Standup" --duration 30

# List cloud recordings from a date range
zoom recordings list --from 2026-01-01

# Get current user info
zoom users me
```

## Configuration

### Config file

`~/.config/zoom-cli/config.toml`

```toml
[default]
account_id = "YOUR_ACCOUNT_ID"
client_id = "YOUR_CLIENT_ID"
client_secret = "YOUR_CLIENT_SECRET"

[work]
account_id = "..."
client_id = "..."
client_secret = "..."
```

Requires a [Zoom Server-to-Server OAuth app](https://marketplace.zoom.us/develop/create).

### Environment variables

| Variable | Description |
|---|---|
| `ZOOM_ACCOUNT_ID` | OAuth account ID |
| `ZOOM_CLIENT_ID` | OAuth client ID |
| `ZOOM_CLIENT_SECRET` | OAuth client secret |
| `ZOOM_PROFILE` | Active profile name (default: `default`) |

### Precedence

CLI flags > environment variables > config file

## Commands

### Meetings

| Command | Description |
|---------|-------------|
| `zoom meetings list [--user <id>] [--type <t>]` | List meetings for a user |
| `zoom meetings get <id>` | Get meeting details |
| `zoom meetings create --topic <t> [--duration <m>] [--start <dt>] [--password <p>]` | Create a meeting |
| `zoom meetings update <id> [--topic <t>] [--duration <m>] [--start <dt>]` | Update a meeting |
| `zoom meetings delete <id>` | Delete a meeting |
| `zoom meetings end <id>` | End a live meeting |
| `zoom meetings participants <id>` | List past meeting participants |
| `zoom meetings invite <id>` | Get meeting invitation text |

### Recordings

| Command | Description |
|---------|-------------|
| `zoom recordings list [--user <id>] [--from <date>] [--to <date>]` | List cloud recordings |
| `zoom recordings get <meeting_id>` | Get recording details |
| `zoom recordings download <meeting_id> [--out <dir>]` | Download recording files |
| `zoom recordings delete <meeting_id> [--permanent]` | Delete cloud recordings |
| `zoom recordings start <meeting_id>` | Start recording a live meeting |
| `zoom recordings stop <meeting_id>` | Stop recording |
| `zoom recordings pause <meeting_id>` | Pause recording |
| `zoom recordings resume <meeting_id>` | Resume recording |
| `zoom recordings transcript <meeting_id> [--out <dir>]` | Download transcript files |

### Users

| Command | Description |
|---------|-------------|
| `zoom users list [--status <s>]` | List account users |
| `zoom users get <id_or_email>` | Get user details |
| `zoom users me` | Get current user |
| `zoom users create --email <e> [--first-name <n>] [--last-name <n>] [--type <t>]` | Create a user |
| `zoom users deactivate <id_or_email>` | Deactivate a user |
| `zoom users activate <id_or_email>` | Reactivate a user |

### Webinars

| Command | Description |
|---------|-------------|
| `zoom webinars list [--user <id>]` | List webinars |
| `zoom webinars get <id>` | Get webinar details |

### Reports

| Command | Description |
|---------|-------------|
| `zoom reports meetings --from <date> [--to <date>] [--user <id>]` | Meeting summary report |
| `zoom reports participants <meeting_id>` | Participant report for a past meeting |

### Configuration & Setup

| Command | Description |
|---------|-------------|
| `zoom init [--profile <name>]` | Set up credentials interactively |
| `zoom config show` | Show current configuration |
| `zoom config delete <profile> [--force]` | Delete a profile |
| `zoom schema <resource>` | Print field reference for a resource (meetings, recordings, users, reports, webinars) |
| `zoom completions <shell>` | Generate shell completions (bash, zsh, fish, elvish, powershell) |

## Shell Completions

```bash
# zsh
zoom completions zsh > ~/.zsh/completions/_zoom

# bash
zoom completions bash > /etc/bash_completion.d/zoom

# fish
zoom completions fish > ~/.config/fish/completions/zoom.fish
```

## Agent Integration

### JSON output

JSON output is automatic when stdout is not a TTY, or forced with `--json`. Data goes to stdout, messages to stderr.

```bash
zoom meetings list --json | jq '.[].topic'
```

### Schema introspection

```bash
zoom schema meetings | jq '.fields'
```

The `schema` command outputs a JSON description of resource fields and types -- enabling AI agents to discover operations without parsing help text.

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | General error |
| `2` | Auth or config error |
| `3` | Not found |

## License

MIT
