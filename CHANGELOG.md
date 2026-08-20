# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog and this project follows Semantic Versioning.

## [0.4.8] - 2026-08-20

### Added
- Diagnostic classification for HTML/non-audio responses and empty streams before decoder startup.
- Support for local `.pls` playlist files in addition to M3U/M3U8.
- Additional regression tests around ICY metadata handling, .pls parsing and failover candidate selection.

### Changed
- Connection recovery for output-device changes now follows the same non-blocking worker model as station startup.
- Release checks now explicitly cover a production build and a smoke test of the packaged binary.
- Playlist browsing now recognizes `.pls` files and loads them through the correct parser.

### Fixed
- Clearer error messages when a station URL returns HTML or an unsupported media type instead of a radio stream.
- Failover and reconnect logic remains responsive without blocking the terminal UI.
- Panic when HTML detection inspected prefix bytes that included invalid UTF-8 sequences.

## [0.4.7] - 2026-08-18

### Added
- Remote PLS/Shoutcast playlist resolution with bounded nesting and playlist size.
- Background audio preparation so network reads and decoder initialization do not block the TUI.
- Cooperative cancellation for obsolete connection attempts and application shutdown.
- Safe recursive playlist scanning that avoids revisiting canonical directories.

### Changed
- Initial playback reuses the single HTTP stream opened by the connection worker.
- Favorites are loaded directly from persisted favorites instead of rescanning all playlists.
- Failed station tracking records every candidate in a failover cycle and clears automatic failures after successful playback.

### Fixed
- UI lockups while opening slow, silent, or unsupported radio streams.
- Loss of the first audio byte during stream probing.
- Stale automatic failure state after a station recovers.

## [0.4.6] - 2026-05-29

### Added
- Playback history screen in Main Menu with direct replay from stored URLs.
- 7-day history retention with accumulated listening time per station.
- Diagnostics file logging for key playback, recovery, and connection events.

### Changed
- Exiting Player with `q`/`Esc` now stops playback.
- Main menu includes a dedicated History option and Help includes History controls.
- CI now triggers on `Main` branch naming.

## [0.4.4] - 2026-05-29

### Added
- Automatic recovery when the system default audio output device changes during playback.

### Changed
- Player now attempts output-device rebind before station failover when playback health degrades after a device switch.
- Help screen documents automatic output recovery behavior and expected micro-cut during rebind.

## [0.4.3] - 2026-05-22

### Added
- Mute/unmute control in Player (`m`) with on-screen mute status.
- Favorite status indicator for the currently playing station.

### Changed
- Favorites persistence now deduplicates entries by stream URL.
- Playlist browser rows now include relative playlist location.
- Player M3U label now includes playlist location context.

## [0.4.0] - 2026-05-21

### Added
- Real-time audio visualizer powered by decoded stream samples.
- Dual gradient bars (`Peak` and `Energy`) in the Player screen.

### Changed
- Player right panel now includes a dedicated bottom `Audio Visualizer` section.
- Visualizer presentation simplified to pure bars (no percentage text).
- Visualizer panel borders are now fully closed for consistent UI framing.

## [0.3.6] - 2026-05-19

### Added
- Persistent volume setting across application restarts
- Stream quality display (bitrate and format) in Player view
- Vertical scrolling in Help screen (j/k, arrows, PgUp/PgDn, Home, End)
- Help hint in Player screen for discoverability
- Explicit Ctrl+C handling for hard-exit support

### Changed
- Player view layout rebalanced to 30/70 split (left panel compacted, right panel expanded)
- Title moved from right panel to left panel for better visual hierarchy
- Control instructions now only appear in Help screen (cleaner Player view)
- Metadata reads non-blocking (try_lock instead of blocking lock)
- ICY metadata parsing moved outside critical section to reduce lock contention

### Fixed
- UI freezes when changing stations during playback (lock contention)
- Help screen overflow on small terminals (vertical scrolling)

## [0.3.5] - 2026-05-13

### Fixed

- Statically link MSVC C runtime (`+crt-static`) to eliminate `VCRUNTIME140.dll` dependency on clean Windows installs.

## [0.3.4] - 2026-05-12

### Added

- Automatic tracking of unreliable streaming URLs to prevent repeated reconnection attempts.
- Unplayable stations marked after 3 consecutive failures and automatically filtered from playlists.

### Changed

- Station selection now skips URLs that have been marked as unplayable due to repeated connection failures.


## [0.3.3] - 2026-05-12

### Added

- Station search now matches both station name and stream URL.
- Favorites-only browsing toggle (`f`) in station browser and player.
- **Architectural milestone**: Custom-built Rust audio engine (rodio + symphonia) replaces mpv dependency.

### Changed

- Replaced mpv-based playback with a custom Rust audio engine (using `rodio` + `symphonia`), eliminating external media player dependencies.
- Improved audio stability with built-in stream health monitoring and automatic failover on stall detection (10s timeout).
- Removed the bottom `Status` panel to use full terminal space for content.
- Updated `cmdRadio v0.3.3` branding in main screens (menu, playlists, stations, player, help, config).

### Fixed

- Station list search and favorites browsing now provide clear, usable selection flow.

## [0.3.1] - 2026-05-12

### Added

- Non-blocking station connection worker for startup/failover attempts.
- Player progress feedback line with spinner (`| / - \\`) while trying stations.

### Changed

- Pressing play from station list now opens Player immediately and shows connection progress.
- Playback state now includes `Connecting` while stream resolution is in progress.

### Fixed

- Removed perceived UI freeze during timeout/retry cycles on unresponsive stations.

## [0.2.0] - 2026-05-11

### Added

- Initial Rust terminal project scaffold.
- User-folder-only runtime storage policy implementation.
- M3U parser and playlist scanner.
- Terminal UI with main menu, playlist browser, station browser, and player screen.
- CI and release workflows for GitHub Actions.
- Player screen playback status (`Playing` / `Paused` / `Stopped`).
- Player screen random mode indicator (`ON` / `OFF`).
- Keyboard volume controls in 5% steps with limits from 0% to 100%.
- Stream startup timeout configuration (`stream_start_timeout_secs`) with default value.

### Changed

- Audio volume clamped to 100% maximum in playback engine.
- Volume display now rounds to stable integer percentages.

### Fixed

- Navigation handling to avoid double movement per key press.
- HTTP radio playback artifacts by separating ICY metadata from audio stream before decoding.
- Automatic station failover: when a station does not respond within timeout, playback advances to next station.

## [0.3.0] - 2026-05-11

### Added

- Little changes.

---

## Version History

This project has evolved across three major technology iterations:

- **v0.1.x — PowerShell** ([original repo](https://github.com/smelpillow/cmdRadio)): Initial implementation using PowerShell and mpv audio backend.
- **v0.2.x — Python** ([Python repo](https://github.com/smelpillow/cmdRadioPy)): Python rewrite maintaining mpv dependency for audio playback.
- **v0.4.x — Rust** (current): Complete architectural redesign with custom-built audio engine (rodio-based), eliminating all external media player dependencies. Improved performance, cross-platform stability, and single-binary distribution.

The Rust implementation (v0.3.0+) represents a fundamental shift toward portability and self-containment, requiring no system-level media player installation.
