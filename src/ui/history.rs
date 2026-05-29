use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::app::{App, HistoryViewItem};

pub fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let rows = app.history_view_items();

    if rows.is_empty() {
        let empty = List::new(vec![ListItem::new("No playback history in the last 7 days")]).block(
            Block::default()
                .title(format!("{} - History (? help)", app.app_title()))
                .borders(Borders::ALL),
        );
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem<'_>> = rows.iter().map(render_row).collect();
    let list = List::new(items)
        .block(
            Block::default()
                .title(format!("{} - History (last 7 days)", app.app_title()))
                .borders(Borders::ALL),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(" > ");

    let mut state = ListState::default();
    state.select(Some(app.history_index()));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_row(item: &HistoryViewItem) -> ListItem<'static> {
    let favorite = if item.is_favorite { "Yes" } else { "No" };
    let duration = format_duration(item.total_duration_secs);
    let date = format_epoch_secs(item.last_played_epoch_secs);
    let line = format!(
        "{} | Fav: {} | Time: {} | Last: {} | {}",
        item.name, favorite, duration, date, item.url
    );

    let style = if item.is_favorite {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    ListItem::new(line).style(style)
}

fn format_duration(total_secs: u64) -> String {
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

fn format_epoch_secs(epoch: u64) -> String {
    // Keep UTC display stable without adding extra dependencies.
    let days = epoch / 86_400;
    let day_secs = epoch % 86_400;
    let hours = day_secs / 3600;
    let minutes = (day_secs % 3600) / 60;
    format!("d{} {:02}:{:02} UTC", days, hours, minutes)
}
