use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let name = app.selected_station_name().unwrap_or("<none>");
    let url = app.selected_station_url().unwrap_or("<none>");
    let text = format!(
        "Now Playing: {name}\nURL: {url}\n\nControls:\nSpace: Play/Pause\nn or Right: Next station\nr: Toggle shuffle\nq: Back to station list"
    );

    let widget = Paragraph::new(text).block(Block::default().title("Player").borders(Borders::ALL));

    frame.render_widget(widget, area);
}
