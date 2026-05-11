use std::path::Path;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::app::{App, Screen};

pub fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    match app.screen {
        Screen::PlaylistBrowser => render_playlists(frame, app, area),
        Screen::StationBrowser => render_stations(frame, app, area),
        _ => {}
    }
}

fn render_playlists(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let items: Vec<ListItem<'_>> = if app.playlists.is_empty() {
        vec![ListItem::new(
            "No .m3u/.m3u8 files found in playlists directory",
        )]
    } else {
        app.playlists
            .iter()
            .map(|path| ListItem::new(file_name(path)))
            .collect()
    };

    let title = format!(
        "Playlists ({}) - Enter open, r shuffle, q back",
        app.config.playlists_dir.display()
    );
    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(" > ");

    let mut state = ListState::default();
    if !app.playlists.is_empty() {
        state.select(Some(app.playlist_index));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_stations(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let items: Vec<ListItem<'_>> = if app.stations.is_empty() {
        vec![ListItem::new("No stations loaded")]
    } else {
        app.stations
            .iter()
            .map(|s| ListItem::new(s.name.clone()))
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title("Stations - Enter play, q back")
                .borders(Borders::ALL),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(" > ");

    let mut state = ListState::default();
    if !app.stations.is_empty() {
        state.select(Some(app.station_index));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| String::from("<unknown>"))
}
