use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub fn render(frame: &mut Frame<'_>, _app: &App, area: Rect) {
    let body = [
        "Global",
        "  ?        Open/close help",
        "  q / Esc  Back (or Exit from main menu)",
        "",
        "Main Menu",
        "  j/k      Move selection",
        "  Enter    Open option",
        "",
        "Playlist Browser",
        "  j/k      Move selection",
        "  Enter    Load playlist",
        "  r        Toggle shuffle",
        "",
        "Station Browser",
        "  j/k      Move selection",
        "  Enter    Play station",
        "  /        Start search mode",
        "  *        Toggle favorite",
        "",
        "Search Mode (Stations)",
        "  Type     Filter stations",
        "  Backsp.  Delete character",
        "  j/k      Move in filtered list",
        "  Enter    Play selected result",
        "  Esc      Exit search mode",
        "",
        "Player",
        "  Space    Play/Pause",
        "  n/Right  Next station",
        "  r        Toggle shuffle",
        "  *        Toggle favorite",
        "  +/-      Volume",
    ]
    .join("\n");

    let paragraph = Paragraph::new(body).block(Block::default().title("Help").borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}
