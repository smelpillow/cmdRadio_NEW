# Architecture

## Modules

- `src/main.rs`: terminal bootstrap, event polling, render loop.
- `src/app.rs`: application state machine and keyboard actions.
- `src/config.rs`: user path resolution, config load/save, writable checks.
- `src/m3u/parser.rs`: parse M3U/M3U8 and collect stations.
- `src/player/audio.rs`: playback abstraction for online streams.
- `src/ui/*`: rendering for each screen.

## Data Flow

1. App startup loads user config and creates user data directories.
2. Playlist browser scans `playlists_dir` for `.m3u/.m3u8` files.
3. Parser turns selected M3U file into `Vec<Station>`.
4. Player screen streams selected station URL and sends audio to default output device.

## State Machine

`MainMenu -> PlaylistBrowser -> StationBrowser -> Player`

`MainMenu -> Config`

## Security / Permissions Model

- Runtime writes are allowed only under user-owned folders.
- App performs startup writability checks.
- No mutable runtime files are written to installation/repository directories.
