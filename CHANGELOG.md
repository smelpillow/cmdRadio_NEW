# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog and this project follows Semantic Versioning.

## [Unreleased]

### Added

- Station search now matches both station name and stream URL.
- Favorites-only browsing toggle (`f`) in station browser and player.

### Changed

- Removed the bottom `Status` panel to use full terminal space for content.
- Added `cmdRadio v0.3.1` branding in main screens (menu, playlists, stations, player, help, config).

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
