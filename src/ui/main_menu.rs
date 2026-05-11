use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::app::App;

pub fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let items = vec![
        ListItem::new("Browse M3U playlists"),
        ListItem::new("Configuration"),
        ListItem::new("Exit"),
    ];

    let list = List::new(items)
        .block(
            Block::default()
                .title("cmdRadio - Main Menu")
                .borders(Borders::ALL),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(" > ");

    let mut state = ListState::default();
    state.select(Some(app.main_menu_index));

    frame.render_stateful_widget(list, area, &mut state);
}
