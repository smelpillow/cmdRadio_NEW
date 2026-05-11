use crossterm::event::KeyCode;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::AppConfig;
use crate::m3u::parser::{Station, parse_m3u_file, scan_m3u_files};
use crate::player::audio::RadioPlayer;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Screen {
    MainMenu,
    PlaylistBrowser,
    StationBrowser,
    Player,
    Config,
    Help,
}

const FAVORITES_FILE_NAME: &str = "favorites.json";
const HISTORY_FILE_NAME: &str = "history.json";
const HISTORY_LIMIT: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HistoryEntry {
    name: String,
    url: String,
    played_at_epoch_secs: u64,
}

pub struct App {
    pub screen: Screen,
    pub status: String,
    pub shuffle: bool,
    pub main_menu_index: usize,
    pub playlist_index: usize,
    pub station_index: usize,
    pub selected_station_index: Option<usize>,
    pub playlists: Vec<PathBuf>,
    pub stations: Vec<Station>,
    pub config: AppConfig,
    previous_screen: Screen,
    station_search_mode: bool,
    station_query: String,
    favorite_urls: HashSet<String>,
    history: Vec<HistoryEntry>,
    player: RadioPlayer,
}

impl App {
    pub fn new() -> Result<Self, String> {
        let config = AppConfig::load_or_create()?;
        config.ensure_directories()?;
        let favorite_urls = Self::load_favorites(&config.data_dir);
        let history = Self::load_history(&config.data_dir);

        let mut app = Self {
            screen: Screen::MainMenu,
            status: String::from("Ready"),
            shuffle: false,
            main_menu_index: 0,
            playlist_index: 0,
            station_index: 0,
            selected_station_index: None,
            playlists: Vec::new(),
            stations: Vec::new(),
            config,
            previous_screen: Screen::MainMenu,
            station_search_mode: false,
            station_query: String::new(),
            favorite_urls,
            history,
            player: RadioPlayer::new()?,
        };

        app.refresh_playlists();
        Ok(app)
    }

    pub fn on_key(&mut self, code: KeyCode) -> bool {
        if code == KeyCode::Char('?') && self.screen != Screen::Help {
            self.previous_screen = self.screen;
            self.screen = Screen::Help;
            return false;
        }

        match self.screen {
            Screen::MainMenu => self.handle_main_menu(code),
            Screen::PlaylistBrowser => self.handle_playlist_browser(code),
            Screen::StationBrowser => self.handle_station_browser(code),
            Screen::Player => self.handle_player(code),
            Screen::Config => self.handle_config(code),
            Screen::Help => self.handle_help(code),
        }
    }

    fn handle_main_menu(&mut self, code: KeyCode) -> bool {
        let menu_len = 3;
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.main_menu_index > 0 {
                    self.main_menu_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.main_menu_index + 1 < menu_len {
                    self.main_menu_index += 1;
                }
            }
            KeyCode::Enter => match self.main_menu_index {
                0 => {
                    self.refresh_playlists();
                    self.screen = Screen::PlaylistBrowser;
                }
                1 => self.screen = Screen::Config,
                2 => return true,
                _ => {}
            },
            KeyCode::Char('q') => return true,
            _ => {}
        }
        false
    }

    fn handle_playlist_browser(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.playlist_index > 0 {
                    self.playlist_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.playlist_index + 1 < self.playlists.len() {
                    self.playlist_index += 1;
                }
            }
            KeyCode::Char('r') => {
                self.shuffle = !self.shuffle;
                self.status = format!("Shuffle {}", if self.shuffle { "ON" } else { "OFF" });
            }
            KeyCode::Enter => {
                if let Some(path) = self.playlists.get(self.playlist_index).cloned() {
                    match parse_m3u_file(&path) {
                        Ok(stations) => {
                            if stations.is_empty() {
                                self.status = String::from("Playlist without stations");
                            } else {
                                self.stations = stations;
                                self.station_index = 0;
                                self.station_search_mode = false;
                                self.station_query.clear();
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
            KeyCode::Esc | KeyCode::Char('q') => self.screen = Screen::MainMenu,
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
            KeyCode::Up | KeyCode::Char('k') => {
                if visible_len > 0 && self.station_index > 0 {
                    self.station_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if visible_len > 0 && self.station_index + 1 < visible_len {
                    self.station_index += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(actual_index) = self.selected_station_browser_index() {
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
            KeyCode::Char('*') => {
                if let Some(actual_index) = self.selected_station_browser_index() {
                    self.toggle_favorite(actual_index);
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
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
            KeyCode::Up | KeyCode::Char('k') => {
                if self.station_index > 0 {
                    self.station_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let visible_len = self.filtered_station_indices().len();
                if visible_len > 0 && self.station_index + 1 < visible_len {
                    self.station_index += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(actual_index) = self.selected_station_browser_index() {
                    self.start_station(actual_index);
                } else {
                    self.status = String::from("No station matches your search");
                }
            }
            KeyCode::Backspace => {
                self.station_query.pop();
                self.clamp_station_cursor();
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
            KeyCode::Char('n') | KeyCode::Right => self.next_station(),
            KeyCode::Char('r') => {
                self.shuffle = !self.shuffle;
                self.status = format!("Shuffle {}", if self.shuffle { "ON" } else { "OFF" });
            }
            KeyCode::Char('*') => {
                if let Some(index) = self.selected_station_index {
                    self.toggle_favorite(index);
                }
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                let new_vol = self.player.adjust_volume(0.05);
                self.status = format!("Volume: {}%", (new_vol * 100.0).round() as u8);
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                let new_vol = self.player.adjust_volume(-0.05);
                self.status = format!("Volume: {}%", (new_vol * 100.0).round() as u8);
            }
            KeyCode::Esc | KeyCode::Char('q') => self.screen = Screen::StationBrowser,
            _ => {}
        }
        false
    }

    fn handle_help(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                self.screen = self.previous_screen;
            }
            _ => {}
        }
        false
    }

    fn handle_config(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Esc | KeyCode::Char('q') => self.screen = Screen::MainMenu,
            KeyCode::Char('b') => match self.config.bootstrap_example_playlist() {
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
            return;
        }

        let timeout = Duration::from_secs(self.config.stream_start_timeout_secs.max(1));
        let mut candidate = index.min(self.stations.len().saturating_sub(1));
        let mut retries_left = self.stations.len().saturating_sub(1);
        let mut last_err: Option<String> = None;

        loop {
            let Some(station) = self.stations.get(candidate).cloned() else {
                self.status = String::from("Invalid station index");
                return;
            };

            match self.player.play_from_url(&station.url, timeout) {
                Ok(()) => {
                    self.selected_station_index = Some(candidate);
                    self.screen = Screen::Player;
                    self.record_history(&station);
                    if let Some(prev_err) = last_err {
                        self.status = format!("Recovered: playing {} after failover ({prev_err})", station.name);
                    } else {
                        self.status = format!("Playing {}", station.name);
                    }
                    return;
                }
                Err(err) => {
                    last_err = Some(err.clone());
                    if retries_left == 0 {
                        self.status = format!(
                            "Playback error after retries (timeout {}s): {err}",
                            self.config.stream_start_timeout_secs.max(1)
                        );
                        return;
                    }

                    retries_left -= 1;
                    candidate = self.next_index_from(candidate);
                }
            }
        }
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

    pub fn playback_state_label(&self) -> &'static str {
        if !self.player.has_active_stream() {
            "Stopped"
        } else if self.player.is_paused() {
            "Paused"
        } else {
            "Playing"
        }
    }

    pub fn volume_percent(&self) -> u8 {
        (self.player.volume() * 100.0).round().clamp(0.0, 100.0) as u8
    }

    pub fn shuffle_label(&self) -> &'static str {
        if self.shuffle { "ON" } else { "OFF" }
    }

    pub fn is_station_search_mode(&self) -> bool {
        self.station_search_mode
    }

    pub fn station_search_query(&self) -> &str {
        &self.station_query
    }

    pub fn filtered_station_indices(&self) -> Vec<usize> {
        if self.station_query.trim().is_empty() {
            return (0..self.stations.len()).collect();
        }

        let query = self.station_query.to_ascii_lowercase();
        self.stations
            .iter()
            .enumerate()
            .filter_map(|(index, station)| {
                let name = station.name.to_ascii_lowercase();
                if name.contains(&query) {
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
            .map(|station| self.favorite_urls.contains(&station.url))
            .unwrap_or(false)
    }

    pub fn favorites_count(&self) -> usize {
        self.favorite_urls.len()
    }

    pub fn history_count(&self) -> usize {
        self.history.len()
    }

    pub fn icy_artist(&self) -> Option<String> {
        self.player.current_metadata().and_then(|m| m.artist)
    }

    pub fn icy_title(&self) -> Option<String> {
        self.player.current_metadata().and_then(|m| m.title)
    }

    fn refresh_playlists(&mut self) {
        self.playlists = scan_m3u_files(&self.config.playlists_dir).unwrap_or_else(|_| Vec::new());
        if self.playlist_index >= self.playlists.len() {
            self.playlist_index = 0;
        }
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
        let Some(station) = self.stations.get(station_index) else {
            return;
        };

        let url = station.url.clone();
        let name = station.name.clone();
        let action = if self.favorite_urls.contains(&url) {
            self.favorite_urls.remove(&url);
            "removed"
        } else {
            self.favorite_urls.insert(url);
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

    fn record_history(&mut self, station: &Station) {
        let played_at_epoch_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.history.retain(|entry| entry.url != station.url);
        self.history.insert(
            0,
            HistoryEntry {
                name: station.name.clone(),
                url: station.url.clone(),
                played_at_epoch_secs,
            },
        );
        if self.history.len() > HISTORY_LIMIT {
            self.history.truncate(HISTORY_LIMIT);
        }

        if let Err(err) = self.save_history() {
            eprintln!("history save failed: {err}");
        }
    }

    fn favorites_path(data_dir: &Path) -> PathBuf {
        data_dir.join(FAVORITES_FILE_NAME)
    }

    fn history_path(data_dir: &Path) -> PathBuf {
        data_dir.join(HISTORY_FILE_NAME)
    }

    fn load_favorites(data_dir: &Path) -> HashSet<String> {
        let path = Self::favorites_path(data_dir);
        let Ok(raw) = fs::read_to_string(path) else {
            return HashSet::new();
        };

        serde_json::from_str::<Vec<String>>(&raw)
            .map(|items| items.into_iter().collect())
            .unwrap_or_default()
    }

    fn save_favorites(&self) -> Result<(), String> {
        let mut items: Vec<String> = self.favorite_urls.iter().cloned().collect();
        items.sort();
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

        serde_json::from_str::<Vec<HistoryEntry>>(&raw).unwrap_or_default()
    }

    fn save_history(&self) -> Result<(), String> {
        let raw = serde_json::to_string_pretty(&self.history)
            .map_err(|e| format!("failed to serialize history: {e}"))?;
        fs::write(Self::history_path(&self.config.data_dir), raw)
            .map_err(|e| format!("failed to write history file: {e}"))
    }
}
