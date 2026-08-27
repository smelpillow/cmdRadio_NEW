pub mod browser;
pub mod help;
pub mod history;
pub mod main_menu;
pub mod player_view;

use ratatui::Frame;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, Screen};

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();

    match app.screen {
        Screen::MainMenu => main_menu::render(frame, app, area),
        Screen::PlaylistBrowser | Screen::StationBrowser => browser::render(frame, app, area),
        Screen::Player => player_view::render(frame, app, area),
        Screen::History => history::render(frame, app, area),
        Screen::Config => render_config(frame, app, area),
        Screen::Help => help::render(frame, app, area),
    }
}

fn render_config(frame: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let body = format!(
        "Config file policy: runtime writes only in user folders.\n\nData: {}\nPlaylists: {}\nCache: {}\nVolume: {}%\n\nKeys: b copy example playlist, ? help, q back",
        app.config.data_dir.display(),
        app.config.playlists_dir.display(),
        app.config.cache_dir.display(),
        (app.config.volume * 100.0).round().clamp(0.0, 100.0) as u8
    );

    let paragraph = Paragraph::new(body).block(
        Block::default()
            .title(format!("{} - Configuration", app.app_title()))
            .borders(Borders::ALL),
    );
    frame.render_widget(paragraph, area);
}
