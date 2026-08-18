# Architecture

## Modules

- `src/main.rs`: terminal bootstrap, event polling, render loop.
- `src/app.rs`: application state machine and keyboard actions.
- `src/config.rs`: user path resolution, config load/save, writable checks.
- `src/m3u/parser.rs`: parse M3U/M3U8 and collect stations.
- `src/player/audio.rs`: HTTP/PLS resolution, decoder preparation, and playback abstraction.
- `src/player/stream.rs`: HTTP and ICY stream adapters with playback progress tracking.
- `src/ui/*`: rendering for each screen.

## Data Flow

1. App startup loads user config and creates user data directories.
2. Playlist browser scans `playlists_dir` for `.m3u/.m3u8` files, canonicalizing directories to avoid cycles.
3. `load_stations_for_playlist()` checks the playlist cache using file size and modification time before parsing an M3U/M3U8 file.
4. The Favorites screen builds stations directly from persisted `favorites.json`; it does not rescan playlists.
5. Selecting a station starts a connection worker. The worker opens one HTTP stream, resolves remote PLS playlists when needed, and prepares the decoder.
6. The worker emits `Attempt`, `Ready`, or `Failure` events. The UI thread only installs the prepared source in the audio sink.
7. An `AtomicBool` cancellation token invalidates obsolete workers when playback is stopped, another transition begins, or the application exits.
8. HTTP streams without ICY metadata use `HttpStream`; ICY streams use `IcyStream`, which removes interleaved metadata before decoding.

Remote PLS resolution accepts HTTP/HTTPS `FileN` entries, reads at most 64 KiB per playlist, and follows at most three nested playlists. Streams are prepared in the worker so slow reads and decoder initialization do not block terminal input.

## State Machine

`MainMenu -> PlaylistBrowser -> StationBrowser -> Player`

`MainMenu -> Config`

## Security / Permissions Model

- Runtime writes are allowed only under user-owned folders.
- App performs startup writability checks.
- No mutable runtime files are written to installation/repository directories.
- Favorites, history, and station health state are persisted under the user data directory; playlist cache data is kept under the user cache directory.
