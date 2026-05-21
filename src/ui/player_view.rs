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

    let state_icon = match state {
        "Playing" => "[>]",
        "Paused" => "[||]",
        "Stopped" => "[x]",
        "Connecting" => "[...]",
        _ => "[?]",
    };

    let mut lines = vec![Line::from(vec![
        Span::styled(format!("{} State: ", state_icon), Style::default().fg(Color::Green)),
        Span::raw(state),
    ])];

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

    let widget = Paragraph::new(lines).block(
        Block::default()
            .title(format!("{} - Player", app.app_title()))
            .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM | Borders::TOP),
    );

    frame.render_widget(widget, area);
}

fn render_right_column(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(6)])
        .split(area);

    let station_name = app.selected_station_name().unwrap_or("<none>");
    let favorite_status = match app.selected_station_is_favorite() {
        Some(true) => ("Yes", Color::Green),
        Some(false) => ("No", Color::Red),
        None => ("N/A", Color::DarkGray),
    };
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
            Span::styled(
                "Station: ",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::raw(station_name),
        ]),
        Line::from(vec![
            Span::styled(
                "Favorite: ",
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            ),
            Span::styled(favorite_status.0, Style::default().fg(favorite_status.1)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Artist: ",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::raw(artist),
        ]),
        Line::from(vec![
            Span::styled(
                "Title: ",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::raw(title),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Bitrate: ",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::raw(bitrate),
        ]),
        Line::from(vec![
            Span::styled(
                "Format: ",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::raw(human_quality),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "URL: ",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::raw(url),
        ]),
        Line::from(vec![
            Span::styled(
                "M3U: ",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw(m3u_name),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Magenta)),
            Span::raw(status),
        ]),
        Line::from(vec![
            Span::styled(
                "Help: ",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw("press ?"),
        ]),
    ];

    let widget = Paragraph::new(lines).block(Block::default().title("Now Playing").borders(Borders::ALL));

    frame.render_widget(widget, right_chunks[0]);
    render_wave_bars(frame, app, right_chunks[1]);
}

fn render_wave_bars(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let (peak_level, energy_level) = app.waveform_levels();
    let bar_width = area.width.saturating_sub(14) as usize;

    let lines = vec![
        gradient_bar_line("Peak", peak_level, bar_width),
        gradient_bar_line("Energy", energy_level, bar_width),
    ];

    let widget = Paragraph::new(lines).block(
        Block::default()
            .title("Audio Visualizer")
            .borders(Borders::ALL),
    );

    frame.render_widget(widget, area);
}

fn gradient_bar_line(label: &str, level: f32, width: usize) -> Line<'static> {
    let safe_width = width.max(12);
    let filled = ((safe_width as f32) * level.clamp(0.0, 1.0)).round() as usize;

    let mut spans = Vec::with_capacity(safe_width + 1);
    spans.push(Span::styled(
        format!("{label:>6}: "),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    ));

    for idx in 0..safe_width {
        if idx < filled {
            spans.push(Span::styled("=", Style::default().fg(gradient_color(idx, safe_width))));
        } else {
            spans.push(Span::styled(".", Style::default().fg(Color::DarkGray)));
        }
    }

    Line::from(spans)
}

fn gradient_color(index: usize, width: usize) -> Color {
    let ratio = if width <= 1 {
        0.0
    } else {
        index as f32 / (width - 1) as f32
    };

    if ratio < 0.5 {
        Color::Green
    } else if ratio < 0.8 {
        Color::Yellow
    } else {
        Color::Red
    }
}
