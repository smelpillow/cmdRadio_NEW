use std::path::Path;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
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
    let filtered = app.filtered_playlist_indices();
    let items: Vec<ListItem<'_>> = if app.playlists.is_empty() {
        vec![ListItem::new(
            "No .m3u/.m3u8 files found in playlists directory",
        )]
    } else if filtered.is_empty() {
        vec![ListItem::new("No playlist matches current search")]
    } else {
        filtered
            .into_iter()
            .filter_map(|i| app.playlists.get(i).map(|path| (i, path)))
            .map(|(i, path)| {
                let name = file_name(path);
                let count_hint = app
                    .playlist_station_count_hint(i)
                    .map(|n| format!(" [{n} st]"))
                    .unwrap_or_default();
                let location = playlist_location(&app.config.playlists_dir, path);
                ListItem::new(format!("{name}{count_hint} - {location}"))
            })
            .collect()
    };

    let mut title = format!(
        "{} - Playlists [{}] - Enter open, / search, PgUp/PgDn page, u refresh, q back",
        app.app_title(),
        app.config.playlists_dir.display()
    );
    if app.is_playlist_search_mode() {
        title = format!(
            "{} - Playlists Search [{}] - type to filter by file/path, PgUp/PgDn page, Esc exit",
            app.app_title(),
            app.playlist_search_query()
        );
    } else if !app.playlist_search_query().is_empty() {
        title = format!(
            "{} - Playlists Filter [{}] - Enter open, / new search, PgUp/PgDn page",
            app.app_title(),
            app.playlist_search_query()
        );
    }

    let has_items = !items.is_empty();

    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" >> ");

    let mut state = ListState::default();
    if has_items {
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

fn playlist_location(playlists_root: &Path, path: &Path) -> String {
    let parent = path.parent().unwrap_or(playlists_root);

    match parent.strip_prefix(playlists_root) {
        Ok(relative) if relative.as_os_str().is_empty() => String::from("playlists/"),
        Ok(relative) => format!("playlists/{}", relative.display()),
        Err(_) => parent.display().to_string(),
    }
}
