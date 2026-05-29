use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static LOG_PATH: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

pub fn init(path: PathBuf) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let store = LOG_PATH.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = store.lock() {
        *guard = Some(path);
    }
}

pub fn info(message: &str) {
    write_line("INFO", message);
}

pub fn warn(message: &str) {
    write_line("WARN", message);
}

pub fn error(message: &str) {
    write_line("ERROR", message);
}

fn write_line(level: &str, message: &str) {
    let Some(path) = current_log_path() else {
        return;
    };

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "[{timestamp}] {level} {message}");
    }
}

fn current_log_path() -> Option<PathBuf> {
    let store = LOG_PATH.get()?;
    let guard = store.lock().ok()?;
    guard.clone()
}
