mod app;
mod config;
mod m3u;
mod player;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::app::App;

fn main() {
    if let Err(err) = run() {
        eprintln!("cmdradio failed: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    enable_raw_mode().map_err(|e| format!("failed to enable raw mode: {e}"))?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)
        .map_err(|e| format!("failed to enter alternate screen: {e}"))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| format!("terminal init failed: {e}"))?;

    let mut app = App::new()?;
    let mut should_quit = false;

    while !should_quit {
        terminal
            .draw(|frame| ui::draw(frame, &app))
            .map_err(|e| format!("draw failed: {e}"))?;

        if event::poll(Duration::from_millis(200)).map_err(|e| format!("poll failed: {e}"))?
            && let Event::Key(key) = event::read().map_err(|e| format!("event read failed: {e}"))?
        {
            should_quit = app.on_key(key.code);
        }
    }

    disable_raw_mode().map_err(|e| format!("failed to disable raw mode: {e}"))?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .map_err(|e| format!("failed to leave alternate screen: {e}"))?;
    terminal
        .show_cursor()
        .map_err(|e| format!("failed to show cursor: {e}"))?;

    Ok(())
}
