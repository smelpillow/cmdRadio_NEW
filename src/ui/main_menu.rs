use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::app::App;

pub fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let items = vec![
        ListItem::new("Browse playlists (M3U/PLS)"),
        ListItem::new("Full random (M3U + station)"),
        ListItem::new("Favorites"),
        ListItem::new("History (last 7 days)"),
        ListItem::new("Configuration"),
        ListItem::new("Exit"),
    ];

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!("{} - Main Menu (? help)", app.app_title()))
                .borders(Borders::ALL),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(" > ");

    let mut state = ListState::default();
    state.select(Some(app.main_menu_index));

    frame.render_stateful_widget(list, area, &mut state);
}
