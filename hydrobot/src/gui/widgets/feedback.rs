
use tui::{
    layout::{Rect},
    style::{Color, Style, },
    widgets::{Block, Borders, },
    widgets::{List, ListItem},
};
use chrono::{Utc, DateTime};
use super::super::*;

pub struct FeedBackWidget {
    selected:bool,
}

impl FeedBackWidget {
    pub fn new() -> Self {
        Self{
            selected: false,
        }
    }
}

impl SelectableWidget for FeedBackWidget {
    fn render(&self, app: &App, frame: &mut Fram, area: Rect) {
        let items: Vec<ListItem> = app.logs
            .iter()
            .rev()
            .map(|(time, msg, level)| {
                let datetime: DateTime<Utc> = DateTime::from(*time);
                ListItem::new(format!("[{}] {}", datetime.format("%d/%m %T"), msg)).style(match level {
                    LogLevel::Error => Style::default().bg(Color::Red),
                    LogLevel::Warn => Style::default().bg(Color::Yellow),
                    LogLevel::Info => Style::default(),
                })
            })
            .collect();
        let items = List::new(items)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(if self.selected {Color::White} else {Color::DarkGray})).title("Feedback"));
        frame.render_widget(items, area);
    }

    fn select(&mut self) {
        self.selected = true;
    }

    fn deselect(&mut self) {
        self.selected = false;
    }
}
