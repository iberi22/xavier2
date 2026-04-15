use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub struct LogStream;

impl Default for LogStream {
    fn default() -> Self {
        Self::new()
    }
}

impl LogStream {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&mut self, f: &mut Frame, logs: &[String], area: Rect) {
        let items: Vec<ListItem> = logs
            .iter()
            .map(|log| {
                let style = if log.contains("S1:") {
                    Style::default().fg(Color::Green)
                } else if log.contains("S2:") {
                    Style::default().fg(Color::Yellow)
                } else if log.contains("S3:") {
                    Style::default().fg(Color::Blue)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(log.clone()).style(style)
            })
            .collect();
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Agent Activity Log (System 1, 2, 3)"),
            )
            .style(Style::default().fg(Color::White));
        f.render_widget(list, area);
    }
}
