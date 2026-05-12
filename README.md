# cmdRadio

Terminal-based online radio player in Rust with M3U playlist support.

## Features

- Main menu and terminal UI navigation.
- Playlist browser for `.m3u` and `.m3u8` files.
- Full random mode from main menu (random M3U + random station).
- Station search by name and URL.
- Favorites support with optional favorites-only filter.
- Player screen with playback state (`Connecting` / `Playing` / `Paused` / `Stopped`).
- Non-blocking station connection attempts with live progress in Player.
- Player screen with visible random mode indicator (`ON` / `OFF`).
- Volume controls from keyboard with live percentage display.
- ICY metadata display (`artist` and `title`) when provided by the stream.
- Player controls for Play/Pause/Next actions.
- Shuffle mode for station selection.
- Runtime files always stored in user directories (safe permissions model).
- No external system dependencies required.

## Runtime File Policy

cmdRadio never writes mutable data in the repository folder or next to the executable.

### Windows

- Config: `%APPDATA%\\cmdradio\\config.toml`
- Data: `%LOCALAPPDATA%\\cmdradio\\`
- Playlists: `%LOCALAPPDATA%\\cmdradio\\playlists\\`
- Cache: `%LOCALAPPDATA%\\cmdradio\\cache\\`

### Linux

- Config: `~/.config/cmdradio/config.toml`
- Data: `~/.local/share/cmdradio/`
- Playlists: `~/.local/share/cmdradio/playlists/`
- Cache: `~/.cache/cmdradio/`

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
