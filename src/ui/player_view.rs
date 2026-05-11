use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let name = app.selected_station_name().unwrap_or("<none>");
    let state = app.playback_state_label();
    let random_mode = app.shuffle_label();
    let volume = app.volume_percent();
    let artist = app.icy_artist().unwrap_or_else(|| String::from("--"));
    let title = app.icy_title().unwrap_or_else(|| String::from("--"));
    let url = app.selected_station_url().unwrap_or("<none>");
    let favorites = app.favorites_count();
    let history = app.history_count();

    let text = format!(
        "Station: {}\nState:   {}\nRandom:  {}\nVolume:  {}%\n\nArtist: {}\nTitle:  {}\n\nURL: {}\n\nFavorites: {}\nHistory:   {}\n\nControls:\nSpace    : Play/Pause\nn / Right: Next station\nr        : Toggle shuffle\n*        : Toggle favorite\n+/-      : Volume +/- 5% (max 100%)\n?        : Help\nq        : Back to station list",
        name, state, random_mode, volume, artist, title, url, favorites, history
    );

    let widget = Paragraph::new(text)
        .block(Block::default().title("Player").borders(Borders::ALL));

    frame.render_widget(widget, area);
}
