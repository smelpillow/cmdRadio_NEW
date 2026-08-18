use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub fn render(frame: &mut Frame<'_>, _app: &App, area: Rect) {
    let body = [
        "Global",
        "  ?        Open/close help",
        "  j/k      Scroll help",
        "  PgUp/Dn  Page help up/down",
        "  q / Esc  Back (or Exit from main menu)",
        "  Note     Letter shortcuts are case-insensitive",
        "",
        "Main Menu",
        "  j/k      Move selection",
        "  Enter    Open option",
        "  Full random picks random M3U and random station",
        "  Favorites opens all saved favorites",
        "  History shows stations played in last 7 days",
        "",
        "Playlist Browser",
        "  j/k      Move selection",
        "  PgUp/Dn  Page up/down",
        "  Enter    Load playlist",
        "  /        Start search mode (file/path)",
        "  u        Refresh playlist scan",
        "  r        Toggle shuffle",
        "",
        "Station Browser",
        "  j/k      Move selection",
        "  Enter    Play station",
        "  /        Start search mode (name/url)",
        "  f        Toggle favorites-only filter",
        "  *        Toggle favorite",
        "",
        "History",
        "  j/k      Move selection",
        "  PgUp/Dn  Page up/down",
        "  Enter    Play selected history URL",
        "  q/Esc    Back to main menu",
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
        "  m        Mute/Unmute",
        "  q/Esc    Stop playback and return to station browser",
        "  Auto     Reconnects on stream stall/end (~10s)",
        "  Auto     Recovers audio output when default device changes (1-2s cut)",
    ]
    .join("\n");

    let paragraph = Paragraph::new(body).block(
        Block::default()
            .title(format!("{} - Help", _app.app_title()))
            .borders(Borders::ALL),
    ).scroll((_app.help_scroll(), 0));
    frame.render_widget(paragraph, area);
}
