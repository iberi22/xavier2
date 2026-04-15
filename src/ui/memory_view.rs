use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

pub struct MemoryView {
    pub state: TableState,
}

impl Default for MemoryView {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryView {
    pub fn new() -> Self {
        Self {
            state: TableState::default(),
        }
    }

    pub fn next(&mut self, items_count: usize) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= items_count - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self, items_count: usize) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    items_count - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn render(
        &mut self,
        f: &mut Frame,
        memories: &[crate::ui::dashboard::MemoryItem],
        area: Rect,
    ) {
        let selected_style = Style::default().add_modifier(Modifier::REVERSED);
        let normal_style = Style::default().bg(Color::Blue);
        let header_cells = ["ID", "Path", "Content"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow)));
        let header = Row::new(header_cells)
            .style(normal_style)
            .height(1)
            .bottom_margin(1);
        let rows = memories.iter().map(|item| {
            let height = 1;
            let cells = vec![
                Cell::from(item.id.clone()),
                Cell::from(item.path.clone()),
                Cell::from(item.content.clone()),
            ];
            Row::new(cells).height(height as u16).bottom_margin(1)
        });
        let t = Table::new(
            rows,
            [
                Constraint::Percentage(10),
                Constraint::Percentage(20),
                Constraint::Percentage(70),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Memory Explorer (j/k to navigate, / to search)"),
        )
        .row_highlight_style(selected_style)
        .highlight_symbol(">> ");
        f.render_stateful_widget(t, area, &mut self.state);
    }
}
