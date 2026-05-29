use crossterm::event::KeyCode;
use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::AppConfig;
use crate::logger;
use crate::m3u::parser::{Station, parse_m3u_file, scan_m3u_files};
use crate::player::audio::RadioPlayer;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Screen {
    MainMenu,
    PlaylistBrowser,
    StationBrowser,
    Player,
    History,
    Config,
    Help,
}

const FAVORITES_FILE_NAME: &str = "favorites.json";
const HISTORY_FILE_NAME: &str = "history.json";
const PLAYLIST_CACHE_FILE_NAME: &str = "playlist_cache.json";
const UNPLAYABLE_STATIONS_FILE_NAME: &str = "unplayable_stations.json";
const HISTORY_LIMIT: usize = 2000;
const HISTORY_RETENTION_SECS: u64 = 7 * 24 * 60 * 60;
const PAGE_STEP: usize = 12;
const PLAYBACK_STALL_TIMEOUT_SECS: u64 = 10;
const OUTPUT_SWITCH_RECOVERY_COOLDOWN_SECS: u64 = 8;
const PLAYLIST_CACHE_MAX_BYTES: u64 = 64 * 1024 * 1024;
const UNPLAYABLE_THRESHOLD: u64 = 3;
const SPINNER_FRAMES: [&str; 4] = ["|", "/", "-", "\\"];
const APP_TITLE: &str = "cmdRadio v0.4.5";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HistoryEntry {
    name: String,
    url: String,
    played_at_epoch_secs: u64,
    #[serde(default)]
    duration_secs: u64,
}

#[derive(Debug, Clone)]
struct PlaybackSession {
    name: String,
    url: String,
    started_at_epoch_secs: u64,
}

#[derive(Debug, Clone)]
pub struct HistoryViewItem {
    pub name: String,
    pub url: String,
    pub last_played_epoch_secs: u64,
    pub total_duration_secs: u64,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FavoriteEntry {
    url: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnplayableStation {
    fail_count: u64,
    last_fail_ts: u64,
    #[serde(default)]
    manual_block: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct UnplayableStationsStore {
    schema_version: u32,
    threshold: u64,
    stations: HashMap<String, UnplayableStation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlaylistCacheEntry {
    modified_epoch_secs: u64,
    file_size: u64,
    stations: Vec<Station>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct PlaylistCacheStore {
    entries: HashMap<String, PlaylistCacheEntry>,
}

enum ConnectEvent {
    Attempt {
        request_id: u64,
        attempt: usize,
        total: usize,
        station_index: usize,
        station_name: String,
    },
    Success {
        request_id: u64,
        station_index: usize,
        recovered_from: Option<String>,
    },
    Failure {
        request_id: u64,
        error: String,
        station_url: String,
    },
}

pub struct App {
    pub screen: Screen,
    pub status: String,
    pub shuffle: bool,
    pub full_random_mode: bool,
    pub main_menu_index: usize,
    pub playlist_index: usize,
    pub station_index: usize,
    pub history_index: usize,
    pub selected_station_index: Option<usize>,
    pub playlists: Vec<PathBuf>,
    pub stations: Vec<Station>,
    pub current_playlist: Option<PathBuf>,
    pub config: AppConfig,
    previous_screen: Screen,
    station_search_mode: bool,
    station_favorites_only: bool,
    station_query: String,
    playlist_search_mode: bool,
    playlist_query: String,
    favorites: Vec<FavoriteEntry>,
    unplayable_stations: UnplayableStationsStore,
    playlist_cache: PlaylistCacheStore,
    history: Vec<HistoryEntry>,
    playback_session: Option<PlaybackSession>,
    help_scroll: u16,
    connection_events: Option<Receiver<ConnectEvent>>,
    active_connect_request_id: Option<u64>,
    next_connect_request_id: u64,
    is_connecting: bool,
    connect_attempt: usize,
    connect_total: usize,
    connect_station_name: String,
    connect_spinner_index: usize,
    last_output_recovery_epoch_secs: Option<u64>,
    is_muted: bool,
    volume_before_mute: f32,
    player: RadioPlayer,
}

impl App {
    pub fn new() -> Result<Self, String> {
        let mut config = AppConfig::load_or_create()?;
        config.ensure_directories()?;
        logger::init(config.diagnostics_log_path());
        logger::info("App initialized");
        let favorites = Self::load_favorites(&config.data_dir);
        let history = Self::load_history(&config.data_dir);
        let unplayable_stations = Self::load_unplayable_stations(&config.data_dir);
        let playlist_cache = Self::load_playlist_cache(&config.data_dir);
        let mut player = RadioPlayer::new()?;
        let normalized_volume = config.volume.clamp(0.0, 1.0);
        player.set_volume(normalized_volume);
        if (config.volume - normalized_volume).abs() > f32::EPSILON {
            config.volume = normalized_volume;
            let _ = config.save();
        }

        let mut app = Self {
            screen: Screen::MainMenu,
            status: String::from("Ready"),
            shuffle: false,
            full_random_mode: false,
            main_menu_index: 0,
            playlist_index: 0,
            station_index: 0,
            history_index: 0,
            selected_station_index: None,
            playlists: Vec::new(),
            stations: Vec::new(),
            current_playlist: None,
            config,
            previous_screen: Screen::MainMenu,
            station_search_mode: false,
            station_favorites_only: false,
            station_query: String::new(),
            playlist_search_mode: false,
            playlist_query: String::new(),
            favorites,
            unplayable_stations,
            playlist_cache,
            history,
            playback_session: None,
            help_scroll: 0,
            connection_events: None,
            active_connect_request_id: None,
            next_connect_request_id: 1,
            is_connecting: false,
            connect_attempt: 0,
            connect_total: 0,
            connect_station_name: String::new(),
            connect_spinner_index: 0,
            last_output_recovery_epoch_secs: None,
            is_muted: false,
            volume_before_mute: normalized_volume.max(0.05),
            player,
        };

        app.refresh_playlists();
        Ok(app)
    }

    pub fn on_tick(&mut self) {
        self.connect_spinner_index = (self.connect_spinner_index + 1) % SPINNER_FRAMES.len();
        self.poll_connection_events();
        self.monitor_playback_health();
    }

    pub fn on_key(&mut self, code: KeyCode) -> bool {
        if code == KeyCode::Char('?') && self.screen != Screen::Help {
            self.previous_screen = self.screen;
            self.help_scroll = 0;
            self.screen = Screen::Help;
            return false;
        }

        match self.screen {
            Screen::MainMenu => self.handle_main_menu(code),
            Screen::PlaylistBrowser => self.handle_playlist_browser(code),
            Screen::StationBrowser => self.handle_station_browser(code),
            Screen::Player => self.handle_player(code),
            Screen::History => self.handle_history(code),
            Screen::Config => self.handle_config(code),
            Screen::Help => self.handle_help(code),
        }
    }

    fn handle_main_menu(&mut self, code: KeyCode) -> bool {
        let menu_len = 6;
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if self.main_menu_index > 0 {
                    self.main_menu_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                if self.main_menu_index + 1 < menu_len {
                    self.main_menu_index += 1;
                }
            }
            KeyCode::Enter => match self.main_menu_index {
                0 => {
                    self.full_random_mode = false;
                    self.playlist_search_mode = false;
                    self.playlist_query.clear();
                    self.clamp_playlist_cursor();
                    self.screen = Screen::PlaylistBrowser;
                }
                1 => self.start_full_random(),
                2 => self.open_favorites_browser(),
                3 => {
                    self.history_index = 0;
                    self.clamp_history_cursor();
                    self.screen = Screen::History;
                }
                4 => self.screen = Screen::Config,
                5 => {
                    self.stop_playback();
                    return true;
                }
                _ => {}
            },
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.stop_playback();
                return true;
            }
            _ => {}
        }
        false
    }

    fn handle_playlist_browser(&mut self, code: KeyCode) -> bool {
        if self.playlist_search_mode {
            return self.handle_playlist_search_mode(code);
        }

        let visible_len = self.filtered_playlist_indices().len();
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if visible_len > 0 && self.playlist_index > 0 {
                    self.playlist_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                if visible_len > 0 && self.playlist_index + 1 < visible_len {
                    self.playlist_index += 1;
                }
            }
            KeyCode::PageUp => {
                self.playlist_index = self.playlist_index.saturating_sub(PAGE_STEP);
            }
            KeyCode::PageDown => {
                if visible_len > 0 {
                    self.playlist_index = (self.playlist_index + PAGE_STEP).min(visible_len - 1);
                }
            }
            KeyCode::Char('/') => {
                self.playlist_search_mode = true;
                self.playlist_query.clear();
                self.playlist_index = 0;
                self.status = String::from("Playlist search mode active. Esc to exit");
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                self.refresh_playlists();
                self.clamp_playlist_cursor();
                self.status = format!("Playlists refreshed: {} files", self.playlists.len());
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.shuffle = !self.shuffle;
                self.status = format!("Shuffle {}", if self.shuffle { "ON" } else { "OFF" });
            }
            KeyCode::Enter => {
                if let Some(actual_index) = self.selected_playlist_browser_index()
                    && let Some(path) = self.playlists.get(actual_index).cloned()
                {
                        match self.load_stations_for_playlist(&path) {
                            Ok(stations) => {
                                if stations.is_empty() {
                                    self.status = String::from("Playlist without stations");
                                } else {
                                    self.full_random_mode = false;
                                    self.stations = stations;
                                    self.station_index = 0;
                                    self.station_search_mode = false;
                                    self.station_favorites_only = false;
                                    self.station_query.clear();
                                    self.playlist_search_mode = false;
                                    self.current_playlist = Some(path.clone());
                                    self.screen = Screen::StationBrowser;
                                    self.status = format!("Loaded {}", path.display());
                                }
                            }
                            Err(err) => {
                                self.status = format!("Cannot parse playlist: {err}");
                            }
                        }
                }
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => self.screen = Screen::MainMenu,
            _ => {}
        }
        false
    }

    fn handle_playlist_search_mode(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if self.playlist_index > 0 {
                    self.playlist_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                let visible_len = self.filtered_playlist_indices().len();
                if visible_len > 0 && self.playlist_index + 1 < visible_len {
                    self.playlist_index += 1;
                }
            }
            KeyCode::PageUp => {
                self.playlist_index = self.playlist_index.saturating_sub(PAGE_STEP);
            }
            KeyCode::PageDown => {
                let visible_len = self.filtered_playlist_indices().len();
                if visible_len > 0 {
                    self.playlist_index = (self.playlist_index + PAGE_STEP).min(visible_len - 1);
                }
            }
            KeyCode::Enter => {
                if let Some(actual_index) = self.selected_playlist_browser_index()
                    && let Some(path) = self.playlists.get(actual_index).cloned()
                {
                        match self.load_stations_for_playlist(&path) {
                            Ok(stations) => {
                                if stations.is_empty() {
                                    self.status = String::from("Playlist without stations");
                                } else {
                                    self.full_random_mode = false;
                                    self.stations = stations;
                                    self.station_index = 0;
                                    self.station_search_mode = false;
                                    self.station_favorites_only = false;
                                    self.station_query.clear();
                                    self.playlist_search_mode = false;
                                    self.current_playlist = Some(path.clone());
                                    self.screen = Screen::StationBrowser;
                                    self.status = format!("Loaded {}", path.display());
                                }
                            }
                            Err(err) => {
                                self.status = format!("Cannot parse playlist: {err}");
                            }
                        }
                } else {
                    self.status = String::from("No playlist matches your search");
                }
            }
            KeyCode::Backspace => {
                self.playlist_query.pop();
                self.clamp_playlist_cursor();
            }
            KeyCode::Char(ch) if !ch.is_control() => {
                self.playlist_query.push(ch);
                self.playlist_index = 0;
            }
            KeyCode::Esc => {
                self.playlist_search_mode = false;
                self.playlist_query.clear();
                self.playlist_index = 0;
                self.status = String::from("Playlist search mode closed");
            }
            _ => {}
        }

        false
    }

    fn handle_station_browser(&mut self, code: KeyCode) -> bool {
        if self.station_search_mode {
            return self.handle_station_search_mode(code);
        }

        let visible_len = self.filtered_station_indices().len();
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if visible_len > 0 && self.station_index > 0 {
                    self.station_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                if visible_len > 0 && self.station_index + 1 < visible_len {
                    self.station_index += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(actual_index) = self.selected_station_browser_index() {
                    self.full_random_mode = false;
                    self.start_station(actual_index);
                } else {
                    self.status = String::from("No stations loaded");
                }
            }
            KeyCode::Char('/') => {
                self.station_search_mode = true;
                self.station_query.clear();
                self.station_index = 0;
                self.status = String::from("Search mode active. Esc to exit");
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.station_favorites_only = !self.station_favorites_only;
                self.station_index = 0;
                self.clamp_station_cursor();
                self.status = format!(
                    "Favorites filter {}",
                    if self.station_favorites_only { "ON" } else { "OFF" }
                );
            }
            KeyCode::Char('*') => {
                if let Some(actual_index) = self.selected_station_browser_index() {
                    self.toggle_favorite(actual_index);
                }
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.station_search_mode = false;
                self.station_query.clear();
                self.station_index = 0;
                self.screen = Screen::PlaylistBrowser;
            }
            _ => {}
        }
        false
    }

    fn handle_station_search_mode(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if self.station_index > 0 {
                    self.station_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                let visible_len = self.filtered_station_indices().len();
                if visible_len > 0 && self.station_index + 1 < visible_len {
                    self.station_index += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(actual_index) = self.selected_station_browser_index() {
                    self.full_random_mode = false;
                    self.start_station(actual_index);
                } else {
                    self.status = String::from("No station matches your search");
                }
            }
            KeyCode::Backspace => {
                self.station_query.pop();
                self.clamp_station_cursor();
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.station_favorites_only = !self.station_favorites_only;
                self.station_index = 0;
                self.clamp_station_cursor();
                self.status = format!(
                    "Favorites filter {}",
                    if self.station_favorites_only { "ON" } else { "OFF" }
                );
            }
            KeyCode::Char('*') => {
                if let Some(actual_index) = self.selected_station_browser_index() {
                    self.toggle_favorite(actual_index);
                }
            }
            KeyCode::Char(ch) if !ch.is_control() => {
                self.station_query.push(ch);
                self.station_index = 0;
            }
            KeyCode::Esc => {
                self.station_search_mode = false;
                self.station_query.clear();
                self.station_index = 0;
                self.status = String::from("Search mode closed");
            }
            _ => {}
        }

        false
    }

    fn handle_player(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char(' ') => match self.player.toggle_pause() {
                Ok(is_paused) => {
                    self.status = if is_paused {
                        String::from("Paused")
                    } else {
                        String::from("Playing")
                    };
                }
                Err(err) => self.status = err,
            },
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Right => {
                if self.full_random_mode {
                    self.start_full_random();
                } else {
                    self.next_station();
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.shuffle = !self.shuffle;
                self.status = format!("Shuffle {}", if self.shuffle { "ON" } else { "OFF" });
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.station_favorites_only = !self.station_favorites_only;
                self.status = format!(
                    "Favorites filter {}",
                    if self.station_favorites_only { "ON" } else { "OFF" }
                );
            }
            KeyCode::Char('*') => {
                if let Some(index) = self.selected_station_index {
                    self.toggle_favorite(index);
                }
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                if self.is_muted {
                    self.restore_unmuted_volume();
                }
                let new_vol = self.player.adjust_volume(0.05);
                self.persist_volume(new_vol);
                self.status = format!("Volume: {}%", (new_vol * 100.0).round() as u8);
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                if self.is_muted {
                    self.restore_unmuted_volume();
                }
                let new_vol = self.player.adjust_volume(-0.05);
                self.persist_volume(new_vol);
                self.status = format!("Volume: {}%", (new_vol * 100.0).round() as u8);
            }
            KeyCode::Char('m') | KeyCode::Char('M') => self.toggle_mute(),
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.stop_playback();
                self.status = String::from("Playback stopped");
                self.screen = Screen::StationBrowser;
            }
            _ => {}
        }
        false
    }

    fn handle_history(&mut self, code: KeyCode) -> bool {
        let visible_len = self.history_view_items().len();
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if visible_len > 0 && self.history_index > 0 {
                    self.history_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                if visible_len > 0 && self.history_index + 1 < visible_len {
                    self.history_index += 1;
                }
            }
            KeyCode::PageUp => {
                self.history_index = self.history_index.saturating_sub(PAGE_STEP);
            }
            KeyCode::PageDown => {
                if visible_len > 0 {
                    self.history_index = (self.history_index + PAGE_STEP).min(visible_len - 1);
                }
            }
            KeyCode::Enter => {
                if let Some(item) = self.selected_history_item() {
                    self.play_url_from_history(&item.name, &item.url);
                } else {
                    self.status = String::from("History is empty");
                }
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.screen = Screen::MainMenu;
            }
            _ => {}
        }
        false
    }

    fn handle_help(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                self.help_scroll = self.help_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                self.help_scroll = self.help_scroll.saturating_add(1);
            }
            KeyCode::PageUp => {
                self.help_scroll = self.help_scroll.saturating_sub(8);
            }
            KeyCode::PageDown => {
                self.help_scroll = self.help_scroll.saturating_add(8);
            }
            KeyCode::Home => {
                self.help_scroll = 0;
            }
            KeyCode::End => {
                self.help_scroll = u16::MAX;
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Char('?') => {
                self.screen = self.previous_screen;
            }
            _ => {}
        }
        false
    }

    pub fn help_scroll(&self) -> u16 {
        self.help_scroll
    }

    fn handle_config(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => self.screen = Screen::MainMenu,
            KeyCode::Char('b') | KeyCode::Char('B') => match self.config.bootstrap_example_playlist() {
                Ok(p) => self.status = format!("Bootstrap copied: {}", p.display()),
                Err(e) => self.status = format!("Bootstrap failed: {e}"),
            },
            _ => {}
        }
        false
    }

    fn start_station(&mut self, index: usize) {
        if self.stations.is_empty() {
            self.status = String::from("No stations loaded");
            logger::warn("start_station requested with empty stations list");
            return;
        }

        self.stop_playback_for_transition();

        let request_id = self.next_connect_request_id;
        self.next_connect_request_id = self.next_connect_request_id.saturating_add(1);
        self.active_connect_request_id = Some(request_id);

        let total = self.stations.len();
        let candidate = index.min(total.saturating_sub(1));
        self.screen = Screen::Player;
        self.is_connecting = true;
        self.connect_attempt = 1;
        self.connect_total = total;
        self.connect_spinner_index = 0;
        self.selected_station_index = Some(candidate);
        self.connect_station_name = self
            .stations
            .get(candidate)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| String::from("<unknown>"));
        self.status = format!("Connecting 1/{}: {}", total, self.connect_station_name);
        logger::info(&format!(
            "starting station connect request id={} station={} total_candidates={}",
            request_id, self.connect_station_name, total
        ));

        let (tx, rx) = mpsc::channel();
        self.connection_events = Some(rx);
        let stations = self.stations.clone();
        let shuffle = self.shuffle;
        let timeout = Duration::from_secs(self.config.stream_start_timeout_secs.max(1));

        thread::spawn(move || {
            let mut candidate = candidate;
            let total = stations.len();
            let mut retries_left = total.saturating_sub(1);
            let mut last_err: Option<String> = None;
            let mut attempt = 1;

            loop {
                let Some(station) = stations.get(candidate).cloned() else {
                    let _ = tx.send(ConnectEvent::Failure {
                        request_id,
                        error: String::from("Invalid station index"),
                        station_url: String::new(),
                    });
                    return;
                };

                let _ = tx.send(ConnectEvent::Attempt {
                    request_id,
                    attempt,
                    total,
                    station_index: candidate,
                    station_name: station.name.clone(),
                });

                match Self::probe_station(&station.url, timeout) {
                    Ok(()) => {
                        let _ = tx.send(ConnectEvent::Success {
                            request_id,
                            station_index: candidate,
                            recovered_from: last_err,
                        });
                        return;
                    }
                    Err(err) => {
                        last_err = Some(err.clone());
                        if retries_left == 0 {
                            let _ = tx.send(ConnectEvent::Failure {
                                request_id,
                                error: format!(
                                    "Playback error after retries (timeout {}s): {err}",
                                    timeout.as_secs().max(1)
                                ),
                                station_url: station.url.clone(),
                            });
                            return;
                        }

                        retries_left -= 1;
                        candidate = Self::next_candidate_index(candidate, total, shuffle);
                        attempt += 1;
                    }
                }
            }
        });
    }

    fn start_full_random(&mut self) {
        self.refresh_playlists();

        let mut playlist_indices: Vec<usize> = (0..self.playlists.len()).collect();
        if playlist_indices.is_empty() {
            self.status = String::from("No playlists available for full random");
            return;
        }

        let mut rng = rand::rng();
        playlist_indices.shuffle(&mut rng);

        for playlist_index in playlist_indices {
            let Some(path) = self.playlists.get(playlist_index).cloned() else {
                continue;
            };

            let stations = match self.load_stations_for_playlist(&path) {
                Ok(stations) if !stations.is_empty() => stations,
                _ => continue,
            };

            let station_index = rng.random_range(0..stations.len());
            let playlist_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("<unknown>");

            self.full_random_mode = true;
            self.stations = stations;
            self.station_index = 0;
            self.station_search_mode = false;
            self.station_favorites_only = false;
            self.station_query.clear();
            self.current_playlist = Some(path.clone());
            self.status = format!("Full random from playlist: {playlist_name}");
            self.start_station(station_index);
            return;
        }

        self.status = String::from("No valid stations found across playlists for full random");
    }

    fn open_favorites_browser(&mut self) {
        self.refresh_playlists();

        if self.favorites.is_empty() {
            self.status = String::from("No favorites saved yet");
            return;
        }

        let mut favorites_list = Vec::new();
        let mut seen_urls = HashSet::new();

        for path in self.playlists.clone() {
            let stations = match self.load_stations_for_playlist(&path) {
                Ok(stations) => stations,
                Err(_) => continue,
            };

            for station in stations {
                if !self.is_url_favorite(&station.url) {
                    continue;
                }

                if seen_urls.insert(station.url.clone()) {
                    favorites_list.push(station);
                }
            }
        }

        if favorites_list.is_empty() {
            self.status = String::from("No favorite stations found in current playlists");
            return;
        }

        favorites_list.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));

        self.full_random_mode = false;
        self.stations = favorites_list;
        self.station_index = 0;
        self.selected_station_index = None;
        self.station_search_mode = false;
        self.station_favorites_only = false;
        self.station_query.clear();
        self.current_playlist = None;
        self.screen = Screen::StationBrowser;
        self.status = format!("Favorites loaded: {} stations", self.stations.len());
    }

    fn next_station(&mut self) {
        if self.stations.is_empty() {
            self.status = String::from("No stations loaded");
            return;
        }

        let current = self.selected_station_index.unwrap_or(0);
        let next_index = self.next_index_from(current);

        self.start_station(next_index);
    }

    fn next_index_from(&self, current: usize) -> usize {
        if self.stations.is_empty() {
            return 0;
        }

        if self.shuffle {
            if self.stations.len() == 1 {
                return 0;
            }
            let mut rng = rand::rng();
            let mut candidate = rng.random_range(0..self.stations.len());
            if candidate == current {
                candidate = (candidate + 1) % self.stations.len();
            }
            candidate
        } else {
            (current + 1) % self.stations.len()
        }
    }

    fn play_url_from_history(&mut self, name: &str, url: &str) {
        self.full_random_mode = false;
        self.station_search_mode = false;
        self.station_favorites_only = false;
        self.station_query.clear();
        self.current_playlist = None;
        self.stations = vec![Station {
            name: name.to_string(),
            url: url.to_string(),
        }];
        self.station_index = 0;
        self.selected_station_index = Some(0);
        self.start_station(0);
    }

    pub fn history_view_items(&self) -> Vec<HistoryViewItem> {
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(HISTORY_RETENTION_SECS);
        let mut merged: HashMap<String, HistoryViewItem> = HashMap::new();

        for entry in &self.history {
            if entry.played_at_epoch_secs < cutoff {
                continue;
            }

            let row = merged.entry(entry.url.clone()).or_insert_with(|| HistoryViewItem {
                name: entry.name.clone(),
                url: entry.url.clone(),
                last_played_epoch_secs: entry.played_at_epoch_secs,
                total_duration_secs: 0,
                is_favorite: self.is_url_favorite(&entry.url),
            });

            row.total_duration_secs = row.total_duration_secs.saturating_add(entry.duration_secs);
            if entry.played_at_epoch_secs > row.last_played_epoch_secs {
                row.last_played_epoch_secs = entry.played_at_epoch_secs;
                row.name = entry.name.clone();
            }
            row.is_favorite = self.is_url_favorite(&entry.url);
        }

        let mut rows: Vec<HistoryViewItem> = merged.into_values().collect();
        rows.sort_by(|a, b| b.last_played_epoch_secs.cmp(&a.last_played_epoch_secs));
        rows
    }

    fn selected_history_item(&self) -> Option<HistoryViewItem> {
        self.history_view_items().get(self.history_index).cloned()
    }

    fn clamp_history_cursor(&mut self) {
        let visible_len = self.history_view_items().len();
        if visible_len == 0 {
            self.history_index = 0;
            return;
        }

        if self.history_index >= visible_len {
            self.history_index = visible_len - 1;
        }
    }

    pub fn history_index(&self) -> usize {
        self.history_index
    }

    pub fn selected_station_name(&self) -> Option<&str> {
        self.selected_station_index
            .and_then(|i| self.stations.get(i))
            .map(|s| s.name.as_str())
    }

    pub fn selected_station_url(&self) -> Option<&str> {
        self.selected_station_index
            .and_then(|i| self.stations.get(i))
            .map(|s| s.url.as_str())
    }

    pub fn selected_station_is_favorite(&self) -> Option<bool> {
        let url = self.selected_station_url()?;
        Some(self.is_url_favorite(url))
    }

    pub fn playback_state_label(&self) -> &'static str {
        if self.is_connecting {
            "Connecting"
        } else if !self.player.has_active_stream() {
            "Stopped"
        } else if self.player.is_paused() {
            "Paused"
        } else {
            "Playing"
        }
    }

    pub fn connection_progress_label(&self) -> Option<String> {
        if !self.is_connecting {
            return None;
        }

        Some(format!(
            "{} Trying {}/{}: {}",
            SPINNER_FRAMES[self.connect_spinner_index],
            self.connect_attempt.max(1),
            self.connect_total.max(1),
            self.connect_station_name
        ))
    }

    pub fn volume_percent(&self) -> u8 {
        (self.player.volume() * 100.0).round().clamp(0.0, 100.0) as u8
    }

    fn persist_volume(&mut self, volume: f32) {
        self.config.volume = volume.clamp(0.0, 1.0);
        if let Err(err) = self.config.save() {
            eprintln!("failed to persist volume in config.toml: {err}");
            logger::warn(&format!("failed to persist volume in config.toml: {err}"));
        }
    }

    fn restore_unmuted_volume(&mut self) -> f32 {
        let restored = self.volume_before_mute.clamp(0.0, 1.0).max(0.05);
        self.is_muted = false;
        self.player.set_volume(restored)
    }

    fn toggle_mute(&mut self) {
        if self.is_muted {
            let restored = self.restore_unmuted_volume();
            self.persist_volume(restored);
            self.status = format!("Unmuted: {}%", (restored * 100.0).round() as u8);
        } else {
            let current = self.player.volume();
            if current > 0.0 {
                self.volume_before_mute = current;
            }
            self.player.set_volume(0.0);
            self.is_muted = true;
            self.status = String::from("Muted");
        }
    }

    pub fn is_muted(&self) -> bool {
        self.is_muted
    }

    pub fn mute_label(&self) -> &'static str {
        if self.is_muted { "ON" } else { "OFF" }
    }

    pub fn shuffle_label(&self) -> &'static str {
        if self.shuffle { "ON" } else { "OFF" }
    }

    pub fn full_random_label(&self) -> &'static str {
        if self.full_random_mode { "ON" } else { "OFF" }
    }

    pub fn current_playlist_with_location_label(&self) -> String {
        let Some(path) = self.current_playlist.as_ref() else {
            return String::from("<none>");
        };

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>");

        let parent = path.parent().unwrap_or(&self.config.playlists_dir);
        let location = match parent.strip_prefix(&self.config.playlists_dir) {
            Ok(relative) if relative.as_os_str().is_empty() => String::from("playlists/"),
            Ok(relative) => format!("playlists/{}", relative.display()),
            Err(_) => parent.display().to_string(),
        };

        format!("{name} - {location}")
    }

    pub fn is_station_search_mode(&self) -> bool {
        self.station_search_mode
    }

    pub fn station_search_query(&self) -> &str {
        &self.station_query
    }

    pub fn station_favorites_only(&self) -> bool {
        self.station_favorites_only
    }

    pub fn is_playlist_search_mode(&self) -> bool {
        self.playlist_search_mode
    }

    pub fn playlist_search_query(&self) -> &str {
        &self.playlist_query
    }

    pub fn filtered_playlist_indices(&self) -> Vec<usize> {
        let query = self.playlist_query.trim().to_ascii_lowercase();
        let has_query = !query.is_empty();

        self.playlists
            .iter()
            .enumerate()
            .filter_map(|(index, path)| {
                if !has_query {
                    return Some(index);
                }

                let path_text = path.to_string_lossy().to_ascii_lowercase();
                let file_text = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();

                if path_text.contains(&query) || file_text.contains(&query) {
                    Some(index)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn playlist_station_count_hint(&self, playlist_index: usize) -> Option<usize> {
        let path = self.playlists.get(playlist_index)?;
        let key = path.to_string_lossy().to_string();
        self.playlist_cache
            .entries
            .get(&key)
            .map(|entry| entry.stations.len())
    }

    pub fn app_title(&self) -> &'static str {
        APP_TITLE
    }

    pub fn filtered_station_indices(&self) -> Vec<usize> {
        let query = self.station_query.trim().to_ascii_lowercase();
        let has_query = !query.is_empty();

        self.stations
            .iter()
            .enumerate()
            .filter_map(|(index, station)| {
                if self.station_favorites_only && !self.is_url_favorite(&station.url) {
                    return None;
                }

                if !has_query {
                    return Some(index);
                }

                let name = station.name.to_ascii_lowercase();
                let url = station.url.to_ascii_lowercase();
                if name.contains(&query) || url.contains(&query) {
                    Some(index)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn is_station_favorite(&self, station_index: usize) -> bool {
        self.stations
            .get(station_index)
            .map(|station| self.is_url_favorite(&station.url))
            .unwrap_or(false)
    }

    pub fn favorites_count(&self) -> usize {
        self.favorites.len()
    }

    fn is_url_favorite(&self, url: &str) -> bool {
        self.favorites.iter().any(|fav| fav.url == url)
    }

    fn normalize_favorites_by_url(favorites: Vec<FavoriteEntry>) -> Vec<FavoriteEntry> {
        let mut seen = HashSet::new();
        let mut normalized = Vec::with_capacity(favorites.len());

        for favorite in favorites.into_iter().rev() {
            if seen.insert(favorite.url.clone()) {
                normalized.push(favorite);
            }
        }

        normalized.reverse();
        normalized
    }

    fn add_favorite(&mut self, station: &Station) {
        if !self.is_url_favorite(&station.url) {
            self.favorites.push(FavoriteEntry {
                url: station.url.clone(),
                name: station.name.clone(),
                source: String::new(),
            });
        }
    }

    fn remove_favorite(&mut self, url: &str) {
        self.favorites.retain(|fav| fav.url != url);
    }

    pub fn icy_artist_title(&self) -> (Option<String>, Option<String>) {
        if let Some(metadata) = self.player.current_metadata() {
            (metadata.artist, metadata.title)
        } else {
            (None, None)
        }
    }

    pub fn stream_bitrate_label(&self) -> String {
        self.player
            .stream_bitrate_kbps()
            .map(|kbps| format!("{kbps} kbps"))
            .unwrap_or_else(|| String::from("Unknown"))
    }

    pub fn stream_human_quality_label(&self) -> String {
        let Some(content_type) = self.player.stream_content_type() else {
            return String::from("Unknown");
        };

        let normalized = content_type.to_ascii_lowercase();

        if normalized.contains("mpeg") || normalized.contains("mp3") {
            return String::from("MP3");
        }
        if normalized.contains("aac") || normalized.contains("m4a") || normalized.contains("mp4") {
            return String::from("AAC");
        }
        if normalized.contains("vorbis") || normalized.contains("ogg") {
            return String::from("OGG/Vorbis");
        }
        if normalized.contains("flac") {
            return String::from("FLAC");
        }
        if normalized.contains("wav") {
            return String::from("WAV");
        }

        content_type.to_string()
    }

    pub fn waveform_levels(&self) -> (f32, f32) {
        self.player.waveform_levels()
    }

    fn refresh_playlists(&mut self) {
        self.playlists = scan_m3u_files(&self.config.playlists_dir).unwrap_or_else(|_| Vec::new());
        self.prune_playlist_cache();
        self.clamp_playlist_cursor();
    }

    fn prune_playlist_cache(&mut self) {
        let valid_keys: HashSet<String> = self
            .playlists
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect();

        let before_len = self.playlist_cache.entries.len();
        self.playlist_cache
            .entries
            .retain(|key, _| valid_keys.contains(key));

        let mut changed = self.playlist_cache.entries.len() != before_len;

        // Guardrail: if cache keeps growing (very large playlists), evict the heaviest entries first.
        while Self::estimate_playlist_cache_size_bytes(&self.playlist_cache) > PLAYLIST_CACHE_MAX_BYTES {
            let Some((largest_key, _)) = self
                .playlist_cache
                .entries
                .iter()
                .max_by_key(|(key, entry)| Self::estimate_cache_entry_size_bytes(key, entry))
            else {
                break;
            };

            let largest_key = largest_key.clone();
            if self.playlist_cache.entries.remove(&largest_key).is_none() {
                break;
            }
            changed = true;
        }

        if changed && let Err(err) = self.save_playlist_cache() {
            eprintln!("playlist cache prune failed: {err}");
        }
    }

    fn estimate_playlist_cache_size_bytes(store: &PlaylistCacheStore) -> u64 {
        store
            .entries
            .iter()
            .map(|(key, entry)| Self::estimate_cache_entry_size_bytes(key, entry))
            .sum()
    }

    fn estimate_cache_entry_size_bytes(key: &str, entry: &PlaylistCacheEntry) -> u64 {
        let base = key.len() as u64 + 128;
        let stations_bytes: u64 = entry
            .stations
            .iter()
            .map(|station| station.name.len() as u64 + station.url.len() as u64 + 32)
            .sum();
        base + stations_bytes
    }

    fn clamp_playlist_cursor(&mut self) {
        let visible_len = self.filtered_playlist_indices().len();
        if visible_len == 0 {
            self.playlist_index = 0;
            return;
        }

        if self.playlist_index >= visible_len {
            self.playlist_index = visible_len - 1;
        }
    }

    fn selected_playlist_browser_index(&self) -> Option<usize> {
        let visible = self.filtered_playlist_indices();
        visible.get(self.playlist_index).copied()
    }

    fn load_stations_for_playlist(&mut self, path: &Path) -> Result<Vec<Station>, String> {
        let signature = Self::playlist_signature(path)?;
        let key = path.to_string_lossy().to_string();

        if let Some(entry) = self.playlist_cache.entries.get(&key)
            && entry.modified_epoch_secs == signature.0
            && entry.file_size == signature.1
        {
            let filtered = entry.stations.iter()
                .filter(|s| !self.is_url_unplayable(&s.url))
                .cloned()
                .collect();
            return Ok(filtered);
        }

        let stations = parse_m3u_file(path)?;
        self.playlist_cache.entries.insert(
            key,
            PlaylistCacheEntry {
                modified_epoch_secs: signature.0,
                file_size: signature.1,
                stations: stations.clone(),
            },
        );

        if let Err(err) = self.save_playlist_cache() {
            eprintln!("playlist cache save failed: {err}");
        }

        let filtered = stations.into_iter()
            .filter(|s| !self.is_url_unplayable(&s.url))
            .collect();
        Ok(filtered)
    }

    fn playlist_signature(path: &Path) -> Result<(u64, u64), String> {
        let meta = fs::metadata(path)
            .map_err(|e| format!("failed to read metadata {}: {e}", path.display()))?;
        let modified_epoch_secs = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Ok((modified_epoch_secs, meta.len()))
    }

    fn clamp_station_cursor(&mut self) {
        let visible_len = self.filtered_station_indices().len();
        if visible_len == 0 {
            self.station_index = 0;
            return;
        }

        if self.station_index >= visible_len {
            self.station_index = visible_len - 1;
        }
    }

    fn selected_station_browser_index(&self) -> Option<usize> {
        let visible = self.filtered_station_indices();
        visible.get(self.station_index).copied()
    }

    fn toggle_favorite(&mut self, station_index: usize) {
        let Some(station) = self.stations.get(station_index).cloned() else {
            return;
        };

        let url = station.url.clone();
        let name = station.name.clone();
        let action = if self.is_url_favorite(&url) {
            self.remove_favorite(&url);
            "removed"
        } else {
            self.add_favorite(&station);
            "added"
        };

        match self.save_favorites() {
            Ok(()) => {
                self.status = format!("Favorite {action}: {name}");
            }
            Err(err) => {
                self.status = format!("Favorite {action} but save failed: {err}");
            }
        }
    }

    fn begin_playback_session(&mut self, station: &Station) {
        let started_at_epoch_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.playback_session = Some(PlaybackSession {
            name: station.name.clone(),
            url: station.url.clone(),
            started_at_epoch_secs,
        });
    }

    fn finalize_playback_session(&mut self) {
        let Some(session) = self.playback_session.take() else {
            return;
        };

        let ended_at_epoch_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let duration_secs = ended_at_epoch_secs
            .saturating_sub(session.started_at_epoch_secs)
            .max(1);

        self.history.insert(
            0,
            HistoryEntry {
                name: session.name,
                url: session.url,
                played_at_epoch_secs: session.started_at_epoch_secs,
                duration_secs,
            },
        );

        self.prune_history_in_place();

        if let Err(err) = self.save_history() {
            eprintln!("history save failed: {err}");
            logger::warn(&format!("history save failed: {err}"));
        }
    }

    fn stop_playback_for_transition(&mut self) {
        self.finalize_playback_session();
        self.player.stop();
        self.abort_connect_request();
    }

    fn stop_playback(&mut self) {
        self.stop_playback_for_transition();
    }

    fn abort_connect_request(&mut self) {
        self.is_connecting = false;
        self.active_connect_request_id = None;
        self.connection_events = None;
    }

    fn prune_history_in_place(&mut self) {
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(HISTORY_RETENTION_SECS);

        self.history.retain(|entry| entry.played_at_epoch_secs >= cutoff);
        if self.history.len() > HISTORY_LIMIT {
            self.history.truncate(HISTORY_LIMIT);
        }
    }

    fn favorites_path(data_dir: &Path) -> PathBuf {
        data_dir.join(FAVORITES_FILE_NAME)
    }

    fn history_path(data_dir: &Path) -> PathBuf {
        data_dir.join(HISTORY_FILE_NAME)
    }

    fn playlist_cache_path(data_dir: &Path) -> PathBuf {
        data_dir.join(PLAYLIST_CACHE_FILE_NAME)
    }

    fn unplayable_stations_path(data_dir: &Path) -> PathBuf {
        data_dir.join(UNPLAYABLE_STATIONS_FILE_NAME)
    }

    fn load_unplayable_stations(data_dir: &Path) -> UnplayableStationsStore {
        let path = Self::unplayable_stations_path(data_dir);
        let Ok(raw) = fs::read_to_string(path) else {
            return UnplayableStationsStore {
                schema_version: 1,
                threshold: UNPLAYABLE_THRESHOLD,
                stations: HashMap::new(),
            };
        };

        serde_json::from_str::<UnplayableStationsStore>(&raw).unwrap_or_else(|_| UnplayableStationsStore {
            schema_version: 1,
            threshold: UNPLAYABLE_THRESHOLD,
            stations: HashMap::new(),
        })
    }

    fn save_unplayable_stations(&self) -> Result<(), String> {
        let mut store = self.unplayable_stations.clone();
        store.schema_version = 1;
        store.threshold = UNPLAYABLE_THRESHOLD;
        let raw = serde_json::to_string_pretty(&store)
            .map_err(|e| format!("failed to serialize unplayable stations: {e}"))?;
        fs::write(Self::unplayable_stations_path(&self.config.data_dir), raw)
            .map_err(|e| format!("failed to write unplayable stations file: {e}"))
    }

    fn is_url_unplayable(&self, url: &str) -> bool {
        self.unplayable_stations.stations
            .get(url)
            .map(|station| station.fail_count >= self.unplayable_stations.threshold || station.manual_block)
            .unwrap_or(false)
    }

    fn record_station_failure(&mut self, url: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let entry = self.unplayable_stations.stations
            .entry(url.to_string())
            .or_insert_with(|| UnplayableStation {
                fail_count: 0,
                last_fail_ts: now,
                manual_block: false,
            });

        entry.fail_count += 1;
        entry.last_fail_ts = now;

        if let Err(err) = self.save_unplayable_stations() {
            eprintln!("unplayable stations save failed: {err}");
        }
    }

    fn load_favorites(data_dir: &Path) -> Vec<FavoriteEntry> {
        let path = Self::favorites_path(data_dir);
        let Ok(raw) = fs::read_to_string(path) else {
            return Vec::new();
        };

        let favorites = serde_json::from_str::<Vec<FavoriteEntry>>(&raw).unwrap_or_default();
        let original_len = favorites.len();
        let normalized = Self::normalize_favorites_by_url(favorites);

        if normalized.len() != original_len
            && let Ok(serialized) = serde_json::to_string_pretty(&normalized)
        {
            let _ = fs::write(Self::favorites_path(data_dir), serialized);
        }

        normalized
    }

    fn save_favorites(&self) -> Result<(), String> {
        let mut items = Self::normalize_favorites_by_url(self.favorites.clone());
        items.sort_by(|a, b| a.url.cmp(&b.url));
        let raw = serde_json::to_string_pretty(&items)
            .map_err(|e| format!("failed to serialize favorites: {e}"))?;
        fs::write(Self::favorites_path(&self.config.data_dir), raw)
            .map_err(|e| format!("failed to write favorites file: {e}"))
    }

    fn load_history(data_dir: &Path) -> Vec<HistoryEntry> {
        let path = Self::history_path(data_dir);
        let Ok(raw) = fs::read_to_string(path) else {
            return Vec::new();
        };

        let mut history = serde_json::from_str::<Vec<HistoryEntry>>(&raw).unwrap_or_default();
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(HISTORY_RETENTION_SECS);

        history.retain(|entry| entry.played_at_epoch_secs >= cutoff);
        if history.len() > HISTORY_LIMIT {
            history.truncate(HISTORY_LIMIT);
        }

        history
    }

    fn save_history(&self) -> Result<(), String> {
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(HISTORY_RETENTION_SECS);

        let mut history: Vec<HistoryEntry> = self
            .history
            .iter()
            .filter(|entry| entry.played_at_epoch_secs >= cutoff)
            .cloned()
            .collect();

        if history.len() > HISTORY_LIMIT {
            history.truncate(HISTORY_LIMIT);
        }

        let raw = serde_json::to_string_pretty(&history)
            .map_err(|e| format!("failed to serialize history: {e}"))?;
        fs::write(Self::history_path(&self.config.data_dir), raw)
            .map_err(|e| format!("failed to write history file: {e}"))
    }

    fn load_playlist_cache(data_dir: &Path) -> PlaylistCacheStore {
        let path = Self::playlist_cache_path(data_dir);
        if let Ok(meta) = fs::metadata(&path)
            && meta.len() > PLAYLIST_CACHE_MAX_BYTES
        {
            let _ = fs::remove_file(&path);
            return PlaylistCacheStore::default();
        }

        let Ok(raw) = fs::read_to_string(path) else {
            return PlaylistCacheStore::default();
        };

        serde_json::from_str::<PlaylistCacheStore>(&raw).unwrap_or_default()
    }

    fn save_playlist_cache(&self) -> Result<(), String> {
        let raw = serde_json::to_string(&self.playlist_cache)
            .map_err(|e| format!("failed to serialize playlist cache: {e}"))?;
        fs::write(Self::playlist_cache_path(&self.config.data_dir), raw)
            .map_err(|e| format!("failed to write playlist cache file: {e}"))
    }

    fn poll_connection_events(&mut self) {
        let Some(active_request_id) = self.active_connect_request_id else {
            return;
        };
        if self.connection_events.is_none() {
            return;
        }

        loop {
            let event = match self.connection_events.as_ref() {
                Some(rx) => rx.try_recv(),
                None => return,
            };

            match event {
                Ok(ConnectEvent::Attempt {
                    request_id,
                    attempt,
                    total,
                    station_index,
                    station_name,
                }) => {
                    if request_id != active_request_id {
                        continue;
                    }
                    self.is_connecting = true;
                    self.connect_attempt = attempt;
                    self.connect_total = total;
                    self.connect_station_name = station_name.clone();
                    self.selected_station_index = Some(station_index);
                    self.status = format!("Connecting {attempt}/{total}: {station_name}");
                }
                Ok(ConnectEvent::Success {
                    request_id,
                    station_index,
                    recovered_from,
                }) => {
                    if request_id != active_request_id {
                        continue;
                    }

                    self.is_connecting = false;
                    self.active_connect_request_id = None;
                    let timeout = Duration::from_secs(self.config.stream_start_timeout_secs.max(1));

                    let Some(station) = self.stations.get(station_index).cloned() else {
                        self.status = String::from("Resolved station is no longer available");
                        self.connection_events = None;
                        return;
                    };

                    match self.player.play_from_url(&station.url, timeout) {
                        Ok(()) => {
                            self.selected_station_index = Some(station_index);
                            self.begin_playback_session(&station);
                            logger::info(&format!(
                                "playback started: station={} url={}",
                                station.name, station.url
                            ));
                            if let Some(prev_err) = recovered_from {
                                self.status = format!(
                                    "Recovered: playing {} after failover ({prev_err})",
                                    station.name
                                );
                            } else {
                                self.status = format!("Playing {}", station.name);
                            }
                        }
                        Err(err) => {
                            logger::error(&format!(
                                "playback start failed after connect: station={} err={}",
                                station.name, err
                            ));
                            self.status = format!("Playback start failed after connect: {err}");
                        }
                    }

                    self.connection_events = None;
                    return;
                }
                Ok(ConnectEvent::Failure { request_id, error, station_url }) => {
                    if request_id != active_request_id {
                        continue;
                    }
                    if !station_url.is_empty() {
                        self.record_station_failure(&station_url);
                    }
                    logger::warn(&format!(
                        "connection failure: station_url={} error={}",
                        station_url, error
                    ));
                    self.is_connecting = false;
                    self.active_connect_request_id = None;
                    self.connection_events = None;
                    self.status = error;
                    return;
                }
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    logger::warn("connection worker disconnected");
                    self.is_connecting = false;
                    self.active_connect_request_id = None;
                    self.connection_events = None;
                    self.status = String::from("Connection worker disconnected");
                    return;
                }
            }
        }
    }

    fn monitor_playback_health(&mut self) {
        if self.is_connecting || self.active_connect_request_id.is_some() {
            return;
        }

        if self.player.is_paused() || !self.player.has_active_stream() {
            return;
        }

        if self.selected_station_index.is_none() {
            return;
        }

        if self.player.default_output_device_changed() {
            self.try_recover_output_switch();
            return;
        }

        if self.player.is_stream_ended() {
            self.trigger_auto_failover("Stream ended");
            return;
        }

        let Some(last_progress) = self.player.last_audio_progress_epoch_secs() else {
            return;
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if now.saturating_sub(last_progress) >= PLAYBACK_STALL_TIMEOUT_SECS {
            self.trigger_auto_failover("Audio stalled");
        }
    }

    fn try_recover_output_switch(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Some(last_recovery) = self.last_output_recovery_epoch_secs
            && now.saturating_sub(last_recovery) < OUTPUT_SWITCH_RECOVERY_COOLDOWN_SECS
        {
            return;
        }

        let Some(station_index) = self.selected_station_index else {
            return;
        };

        let Some(station) = self.stations.get(station_index).cloned() else {
            return;
        };

        self.last_output_recovery_epoch_secs = Some(now);
        self.status = String::from("Audio output changed. Reconnecting current station...");
        logger::warn(&format!(
            "output device switch detected, attempting reconnect: station={} url={}",
            station.name, station.url
        ));

        self.player.invalidate_output_device();
        let timeout = Duration::from_secs(self.config.stream_start_timeout_secs.max(1));

        match self.player.play_from_url(&station.url, timeout) {
            Ok(()) => {
                self.selected_station_index = Some(station_index);
                logger::info(&format!("output device reconnected successfully: {}", station.name));
                self.status = format!("Audio output reconnected: {}", station.name);
            }
            Err(err) => {
                logger::error(&format!(
                    "output device reconnect failed: station={} err={}",
                    station.name, err
                ));
                self.status = format!(
                    "Audio output reconnect failed ({err}). Waiting before retry..."
                );
            }
        }
    }

    fn trigger_auto_failover(&mut self, reason: &str) {
        let station_name = self
            .selected_station_name()
            .unwrap_or("<unknown>")
            .to_string();

        if self.full_random_mode {
            logger::warn(&format!(
                "auto-failover triggered in full-random mode: reason={} station={}",
                reason, station_name
            ));
            self.status = format!(
                "{reason} on {station_name}. Auto-reconnect: full random next pick"
            );
            self.start_full_random();
            return;
        }

        if self.stations.len() <= 1 {
            self.stop_playback();
            logger::warn(&format!(
                "auto-failover aborted, no alternative station: reason={} station={}",
                reason, station_name
            ));
            self.status = format!(
                "{reason} on {station_name}. No alternative station available"
            );
            return;
        }

        logger::warn(&format!(
            "auto-failover to next station: reason={} station={}",
            reason, station_name
        ));
        self.status = format!("{reason} on {station_name}. Auto-reconnect to next station");
        self.next_station();
    }

    fn probe_station(url: &str, timeout: Duration) -> Result<(), String> {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(timeout)
            .timeout_read(timeout)
            .timeout_write(timeout)
            .build();

        let response = agent
            .get(url)
            .set("Icy-MetaData", "1")
            .call()
            .map_err(|e| format!("http request failed: {e}"))?;

        let mut reader = response.into_reader();
        let mut probe = [0_u8; 1];
        reader
            .read_exact(&mut probe)
            .map_err(|e| format!("stream probe failed: {e}"))?;

        Ok(())
    }

    fn next_candidate_index(current: usize, len: usize, shuffle: bool) -> usize {
        if len == 0 {
            return 0;
        }

        if shuffle {
            if len == 1 {
                return 0;
            }
            let mut rng = rand::rng();
            let mut candidate = rng.random_range(0..len);
            if candidate == current {
                candidate = (candidate + 1) % len;
            }
            candidate
        } else {
            (current + 1) % len
        }
    }
}
