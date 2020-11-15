use tui::{
    layout::{Rect},
    style::{Color, Style},
    widgets::{Block, Borders},
    widgets::{List, ListItem },
};
use super::super::*;

pub struct QueryWidget {
    selected:bool,
}

impl QueryWidget {
    pub fn new() -> Self {
        Self{
            selected: false,
        }
    }
}

impl SelectableWidget for QueryWidget {
    fn render(&self, app: &App, frame: &mut Fram, area: Rect) {

        let items: Vec<ListItem> = app.queries
            .iter()
            .rev()
            .map(|(date, msg)| {
                let datetime: chrono::DateTime<chrono::Utc> = chrono::DateTime::from(*date);
                ListItem::new(format!("[{}]{}", datetime.format("%d/%m %T"), msg)).style(Style::default().fg(Color::Black).bg(Color::White))
            })
            .collect();
        let items = List::new(items)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(if self.selected {Color::White} else {Color::DarkGray})).title("Queries"));
        frame.render_widget(items, area);
    }

    fn select(&mut self) {
        self.selected = true;
    }

    fn deselect(&mut self) {
        self.selected = false;
    }
}