use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub fn render(frame: &mut Frame<'_>, _app: &App, area: Rect) {
    let body = [
        "Global",
        "  ?        Open/close help",
        "  q / Esc  Back (or Exit from main menu)",
        "  Note     Letter shortcuts are case-insensitive",
        "",
        "Main Menu",
        "  j/k      Move selection",
        "  Enter    Open option",
        "  Full random picks random M3U and random station",
        "",
        "Playlist Browser",
        "  j/k      Move selection",
        "  Enter    Load playlist",
        "  r        Toggle shuffle",
        "",
        "Station Browser",
        "  j/k      Move selection",
        "  Enter    Play station",
        "  /        Start search mode (name/url)",
        "  f        Toggle favorites-only filter",
        "  *        Toggle favorite",
        "",
        "Search Mode (Stations)",
        "  Type     Filter stations by name/url",
        "  Backsp.  Delete character",
        "  j/k      Move in filtered list",
        "  Enter    Play selected result",
        "  f        Toggle favorites-only filter",
        "  Esc      Exit search mode",
        "",
        "Player",
        "  Space    Play/Pause",
        "  n/Right  Next station (or next full-random pick)",
        "  r        Toggle shuffle",
        "  *        Toggle favorite",
        "  +/-      Volume",
    ]
    .join("\n");

    let paragraph = Paragraph::new(body).block(
        Block::default()
            .title(format!("{} - Help", _app.app_title()))
            .borders(Borders::ALL),
    );
    frame.render_widget(paragraph, area);
}
