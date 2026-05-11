use crossterm::event::KeyCode;
use rand::Rng;

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
}

pub struct App {
    pub screen: Screen,
    pub status: String,
    pub shuffle: bool,
    pub main_menu_index: usize,
    pub playlist_index: usize,
    pub station_index: usize,
    pub selected_station_index: Option<usize>,
    pub playlists: Vec<std::path::PathBuf>,
    pub stations: Vec<Station>,
    pub config: AppConfig,
    player: RadioPlayer,
}

impl App {
    pub fn new() -> Result<Self, String> {
        let config = AppConfig::load_or_create()?;
        config.ensure_directories()?;

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
            player: RadioPlayer::new()?,
        };

        app.refresh_playlists();
        Ok(app)
    }

    pub fn on_key(&mut self, code: KeyCode) -> bool {
        match self.screen {
            Screen::MainMenu => self.handle_main_menu(code),
            Screen::PlaylistBrowser => self.handle_playlist_browser(code),
            Screen::StationBrowser => self.handle_station_browser(code),
            Screen::Player => self.handle_player(code),
            Screen::Config => self.handle_config(code),
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
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.station_index > 0 {
                    self.station_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.station_index + 1 < self.stations.len() {
                    self.station_index += 1;
                }
            }
            KeyCode::Enter => {
                self.start_station(self.station_index);
            }
            KeyCode::Esc | KeyCode::Char('q') => self.screen = Screen::PlaylistBrowser,
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
            KeyCode::Char('+') | KeyCode::Char('=') => {
                let new_vol = self.player.adjust_volume(0.05);
                self.status = format!("Volume: {}%", (new_vol * 100.0) as u8);
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                let new_vol = self.player.adjust_volume(-0.05);
                self.status = format!("Volume: {}%", (new_vol * 100.0) as u8);
            }
            KeyCode::Esc | KeyCode::Char('q') => self.screen = Screen::StationBrowser,
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
        if let Some(station) = self.stations.get(index) {
            match self.player.play_from_url(&station.url) {
                Ok(()) => {
                    self.selected_station_index = Some(index);
                    self.screen = Screen::Player;
                    self.status = format!("Playing {}", station.name);
                }
                Err(err) => {
                    self.status = format!("Playback error: {err}");
                }
            }
        }
    }

    fn next_station(&mut self) {
        if self.stations.is_empty() {
            self.status = String::from("No stations loaded");
            return;
        }

        let next_index = if self.shuffle {
            let mut rng = rand::rng();
            if self.stations.len() == 1 {
                0
            } else {
                let mut candidate = rng.random_range(0..self.stations.len());
                if Some(candidate) == self.selected_station_index {
                    candidate = (candidate + 1) % self.stations.len();
                }
                candidate
            }
        } else {
            let current = self.selected_station_index.unwrap_or(0);
            (current + 1) % self.stations.len()
        };

        self.start_station(next_index);
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
        (self.player.volume() * 100.0) as u8
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
}
