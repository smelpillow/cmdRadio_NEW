use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    // Two-column layout: 30/70 split
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    render_left_column(frame, app, chunks[0]);
    render_right_column(frame, app, chunks[1]);
}

fn render_left_column(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let state = app.playback_state_label();
    let volume = app.volume_percent();
    let random_mode = app.shuffle_label();
    let full_random_mode = app.full_random_label();
    let favorites_filter = if app.station_favorites_only() { "ON" } else { "OFF" };
    let favorites = app.favorites_count();

    // State icon mapping
    let state_icon = match state {
        "Playing" => "[>]",
        "Paused" => "[||]",
        "Stopped" => "[x]",
        "Connecting" => "[...]",
        _ => "[?]",
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled(format!("{} State: ", state_icon), Style::default().fg(Color::Green)),
            Span::raw(state),
        ]),
    ];

    // Connection progress if connecting
    if let Some(progress) = app.connection_progress_label() {
        lines.push(Line::from(Span::styled(
            format!("  {}", progress),
            Style::default().fg(Color::Yellow),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("[R] Random: ", Style::default().fg(Color::Cyan)),
        Span::raw(random_mode),
    ]));
    lines.push(Line::from(vec![
        Span::styled("[F] FullRnd: ", Style::default().fg(Color::Cyan)),
        Span::raw(full_random_mode),
    ]));
    lines.push(Line::from(vec![
        Span::styled("[*] FavFilter: ", Style::default().fg(Color::Cyan)),
        Span::raw(favorites_filter),
    ]));
    lines.push(Line::from(""));

    let volume_bar = format!("{}%", volume);
    lines.push(Line::from(vec![
        Span::styled("Volume: ", Style::default().fg(Color::Cyan)),
        Span::styled(volume_bar, Style::default().fg(Color::Yellow)),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("[*] Favorites: ", Style::default().fg(Color::Magenta)),
        Span::raw(favorites.to_string()),
    ]));

    let widget = Paragraph::new(lines)
        .block(
            Block::default()
                .title(format!("{} - Player", app.app_title()))
                .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM | Borders::TOP),
        );

    frame.render_widget(widget, area);
}

fn render_right_column(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let station_name = app.selected_station_name().unwrap_or("<none>");
    let (artist, title) = app.icy_artist_title();
    let artist = artist.unwrap_or_else(|| String::from("--"));
    let title = title.unwrap_or_else(|| String::from("--"));
    let bitrate = app.stream_bitrate_label();
    let human_quality = app.stream_human_quality_label();
    let url = app.selected_station_url().unwrap_or("<none>");
    let m3u_name = app.current_playlist_label();
    let status = app.status.as_str();

    let lines = vec![
        Line::from(vec![
            Span::styled("Station: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(station_name),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Artist: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(artist),
        ]),
        Line::from(vec![
            Span::styled("Title: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(title),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Bitrate: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(bitrate),
        ]),
        Line::from(vec![
            Span::styled("Format: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(human_quality),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("URL: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(url),
        ]),
        Line::from(vec![
            Span::styled("M3U: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(m3u_name),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Magenta)),
            Span::raw(status),
        ]),
        Line::from(vec![
            Span::styled("Help: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("press ?"),
        ]),
    ];

    let widget = Paragraph::new(lines)
        .block(Block::default().title("Now Playing").borders(Borders::ALL));

    frame.render_widget(widget, area);
}
