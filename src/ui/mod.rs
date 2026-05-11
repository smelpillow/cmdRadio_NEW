pub mod browser;
pub mod main_menu;
pub mod player_view;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, Screen};

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(frame.area());

    match app.screen {
        Screen::MainMenu => main_menu::render(frame, app, chunks[0]),
        Screen::PlaylistBrowser | Screen::StationBrowser => browser::render(frame, app, chunks[0]),
        Screen::Player => player_view::render(frame, app, chunks[0]),
        Screen::Config => render_config(frame, app, chunks[0]),
    }

    let status = Paragraph::new(app.status.clone()).block(
        Block::default()
            .title("Status")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::LightBlue)),
    );
    frame.render_widget(status, chunks[1]);
}

fn render_config(frame: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let body = format!(
        "Config file policy: runtime writes only in user folders.\n\nData: {}\nPlaylists: {}\nCache: {}\n\nKeys: b copy example playlist, q back",
        app.config.data_dir.display(),
        app.config.playlists_dir.display(),
        app.config.cache_dir.display()
    );

    let paragraph = Paragraph::new(body).block(
        Block::default()
            .title("Configuration")
            .borders(Borders::ALL),
    );
    frame.render_widget(paragraph, area);
}
