use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const APP_NAME: &str = "cmdradio";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub playlists_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default = "default_stream_start_timeout_secs")]
    pub stream_start_timeout_secs: u64,
}

impl AppConfig {
    pub fn load_or_create() -> Result<Self, String> {
        let paths = AppPaths::detect()?;
        fs::create_dir_all(&paths.config_dir).map_err(|e| {
            format!(
                "failed to create config directory {}: {e}",
                paths.config_dir.display()
            )
        })?;

        if paths.config_file.exists() {
            let raw = fs::read_to_string(&paths.config_file).map_err(|e| {
                format!(
                    "failed reading config file {}: {e}",
                    paths.config_file.display()
                )
            })?;
            toml::from_str::<Self>(&raw).map_err(|e| {
                format!(
                    "invalid config format in {}: {e}",
                    paths.config_file.display()
                )
            })
        } else {
            let cfg = Self {
                playlists_dir: paths.playlists_dir.clone(),
                data_dir: paths.data_dir.clone(),
                cache_dir: paths.cache_dir.clone(),
                volume: default_volume(),
                stream_start_timeout_secs: default_stream_start_timeout_secs(),
            };
            cfg.save()?;
            Ok(cfg)
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let paths = AppPaths::detect()?;
        fs::create_dir_all(&paths.config_dir).map_err(|e| {
            format!(
                "failed to create config directory {}: {e}",
                paths.config_dir.display()
            )
        })?;

        let raw =
            toml::to_string_pretty(self).map_err(|e| format!("failed to serialize config: {e}"))?;
        fs::write(&paths.config_file, raw).map_err(|e| {
            format!(
                "failed writing config file {}: {e}",
                paths.config_file.display()
            )
        })
    }

    pub fn ensure_directories(&self) -> Result<(), String> {
        Self::ensure_writable_dir(&self.data_dir)?;
        Self::ensure_writable_dir(&self.playlists_dir)?;
        Self::ensure_writable_dir(&self.cache_dir)
    }

    pub fn bootstrap_example_playlist(&self) -> Result<PathBuf, String> {
        let source = Path::new("assets").join("bootstrap").join("example.m3u");

        if !source.exists() {
            return Err(format!("bootstrap source not found: {}", source.display()));
        }

        let target = self.playlists_dir.join("example.m3u");
        if !target.exists() {
            fs::copy(&source, &target).map_err(|e| {
                format!(
                    "failed to copy bootstrap playlist from {} to {}: {e}",
                    source.display(),
                    target.display()
                )
            })?;
        }
        Ok(target)
    }

    fn ensure_writable_dir(path: &Path) -> Result<(), String> {
        fs::create_dir_all(path)
            .map_err(|e| format!("failed to create directory {}: {e}", path.display()))?;

        let test_file = path.join(".write_test");
        fs::write(&test_file, b"ok")
            .map_err(|e| format!("directory not writable {}: {e}", path.display()))?;
        fs::remove_file(&test_file)
            .map_err(|e| format!("failed cleanup in {}: {e}", path.display()))?;
        Ok(())
    }
}

fn default_stream_start_timeout_secs() -> u64 {
    8
}

fn default_volume() -> f32 {
    1.0
}

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub data_dir: PathBuf,
    pub playlists_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl AppPaths {
    pub fn detect() -> Result<Self, String> {
        if cfg!(target_os = "windows") {
            let appdata = env::var_os("APPDATA")
                .map(PathBuf::from)
                .ok_or_else(|| String::from("APPDATA is not set"))?;
            let local_appdata = env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .ok_or_else(|| String::from("LOCALAPPDATA is not set"))?;

            let config_dir = appdata.join(APP_NAME);
            let data_dir = local_appdata.join(APP_NAME);
            let playlists_dir = data_dir.join("playlists");
            let cache_dir = data_dir.join("cache");
            let config_file = config_dir.join("config.toml");

            Ok(Self {
                config_dir,
                config_file,
                data_dir,
                playlists_dir,
                cache_dir,
            })
        } else {
            let home = env::var_os("HOME")
                .map(PathBuf::from)
                .ok_or_else(|| String::from("HOME is not set"))?;

            let config_dir = home.join(".config").join(APP_NAME);
            let data_dir = home.join(".local").join("share").join(APP_NAME);
            let playlists_dir = data_dir.join("playlists");
            let cache_dir = home.join(".cache").join(APP_NAME);
            let config_file = config_dir.join("config.toml");

            Ok(Self {
                config_dir,
                config_file,
                data_dir,
                playlists_dir,
                cache_dir,
            })
        }
    }
}
