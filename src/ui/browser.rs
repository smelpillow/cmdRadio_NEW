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
        "{} - Playlists ({}) - Enter open, r shuffle, ? help, q back",
        app.app_title(),
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
    let filtered = app.filtered_station_indices();
    let items: Vec<ListItem<'_>> = if app.stations.is_empty() {
        vec![ListItem::new("No stations loaded")]
    } else if filtered.is_empty() {
        vec![ListItem::new("No station matches current search")]
    } else {
        filtered
            .into_iter()
            .filter_map(|i| app.stations.get(i).map(|s| (i, s)))
            .map(|(i, s)| {
                let marker = if app.is_station_favorite(i) { "* " } else { "  " };
                ListItem::new(format!("{marker}{}", s.name))
            })
            .collect()
    };

    let mut title = format!(
        "{} - Stations - Enter play, / search, f favorites, * favorite, ? help, q back",
        app.app_title()
    );
    if app.is_station_search_mode() {
        title = format!(
            "{} - Stations Search [{}] - type to filter (name/url), f favorites, Esc exit",
            app.app_title(),
            app.station_search_query()
        );
    } else if !app.station_search_query().is_empty() {
        title = format!(
            "{} - Stations Filter [{}] - Enter play, / new search, f favorites, * favorite",
            app.app_title(),
            app.station_search_query()
        );
    } else if app.station_favorites_only() {
        title = format!(
            "{} - Stations [Favorites only] - Enter play, / search, f favorites, * favorite",
            app.app_title()
        );
    }

    let has_items = !items.is_empty();

    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(" > ");

    let mut state = ListState::default();
    if has_items {
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
