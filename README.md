# cmdRadio

Terminal-based online radio player in Rust with M3U playlist support.

## Features

- Main menu and terminal UI navigation.
- Playlist browser for `.m3u` and `.m3u8` files.
- Player screen with Play/Pause/Next actions.
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

- `j` / `k` or arrow keys: navigate
- `Enter`: select
- `Space`: play/pause
- `n` or right arrow: next station
- `r`: toggle shuffle
- `q` / `Esc`: back/exit

## Development

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Releases

See `docs/RELEASING.md`.
