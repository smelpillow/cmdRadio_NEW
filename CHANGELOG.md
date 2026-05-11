# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog and this project follows Semantic Versioning.

## [Unreleased]

### Added

### Changed

### Fixed

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
