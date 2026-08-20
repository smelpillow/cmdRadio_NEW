# cmdRadio

Terminal-based online radio player in Rust with M3U playlist support.

## Project Evolution

**cmdRadio** started as a PowerShell script (see [original repo](https://github.com/smelpillow/cmdRadio)) and later evolved into a Python implementation (see [Python repo](https://github.com/smelpillow/cmdRadioPy)), both relying on **mpv** as the audio backend. This **Rust version** marks a significant architectural shift: it replaces the external mpv dependency with a **custom-built audio player engine** using `rodio`, eliminating all system-level media player dependencies while improving performance and portability. No external tools required—just pure Rust.

## Features

- Main menu and terminal UI navigation.
- Playlist browser for `.m3u`, `.m3u8`, and `.pls` files.
- Safe recursive playlist scanning with protection against directory cycles.
- Full random mode from main menu (random M3U + random station).
- Station search by name and URL.
- Favorites support with optional favorites-only filter; the Favorites screen loads directly from persisted favorites.
- Player screen with playback state (`Connecting` / `Playing` / `Paused` / `Stopped`).
- Background connection and decoder preparation with live progress in Player.
- Cooperative cancellation when leaving Player, changing station, or exiting the application.
- Remote PLS/Shoutcast playlist resolution with bounded playlist size and nesting depth.
- Single stream opening for initial playback, avoiding a duplicate HTTP connection.
- Better diagnostics for non-audio HTML responses, empty streams, and unsupported content types.
- Player screen with visible random mode indicator (`ON` / `OFF`).
- Volume controls from keyboard with live percentage display.
- ICY metadata display (`artist` and `title`) when provided by the stream.
- Player controls for Play/Pause/Next actions.
- Shuffle mode for station selection.
- Runtime files always stored in user directories (safe permissions model).
- No external system dependencies required.

## Recent reliability improvements

- Connection and decoder work move off the TUI thread so slow or unsupported stations no longer freeze the interface.
- Failover tracking records every failed candidate in a retry chain and clears stale failures after a successful recovery.
- The app detects PLS/Shoutcast playlist responses and resolves them recursively up to a safe depth limit.
- ICY metadata parsing strips interleaved metadata from audio before decoding, preserving the first audio bytes correctly.
- Output device changes trigger a safe reconnect flow without blocking keyboard input.

## Runtime File Policy

cmdRadio never writes mutable data in the repository folder or next to the executable.

### Windows

- Config: `%APPDATA%\\cmdradio\\config.toml`
- Data: `%LOCALAPPDATA%\\cmdradio\\`
- Playlists: `%LOCALAPPDATA%\\cmdradio\\playlists\\`
- Cache: `%LOCALAPPDATA%\\cmdradio\\cache\\`

Persistent data files are stored under the Data directory, including `favorites.json`, `history.json`, and `unplayable_stations.json`. Playlist cache data is stored under the Cache directory.

### Linux

- Config: `~/.config/cmdradio/config.toml`
- Data: `~/.local/share/cmdradio/`
- Playlists: `~/.local/share/cmdradio/playlists/`
- Cache: `~/.cache/cmdradio/`

Persistent data files are stored under the Data directory, including `favorites.json`, `history.json`, and `unplayable_stations.json`. Playlist cache data is stored under the Cache directory.

## Quick Start

```bash
cargo run
```

Then open Configuration and press `b` once to copy the sample playlist into your user playlist folder.

## Keyboard Controls

All alphabetic shortcuts are case-insensitive (`n`/`N`, `q`/`Q`, etc.).

- `j` / `k` or arrow keys: navigate
- `Enter`: select
- `/`: search in station list (name/url)
- `f`: toggle favorites-only filter in stations/player
- `*`: toggle favorite station
- `Space`: play/pause
- `n` or right arrow: next station (in Full Random mode: next random M3U + station)
- `r`: toggle shuffle
- `+` / `=`: volume up (+5%, max 100%)
- `-` / `_`: volume down (-5%, min 0%)
- `q` / `Esc`: back/exit

## Development

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Releases

See `docs/RELEASING.md`.
