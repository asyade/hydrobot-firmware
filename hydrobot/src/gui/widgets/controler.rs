
use termion::{event::Key};
use tui::{
    layout::{Rect},
    style::{Color, Style},
    widgets::{Block, Borders},
    widgets::{List, ListItem },
};
use super::super::*;

pub struct ControlerWidget {
    selected:bool,
    sub: Vec<(&'static str, bool, Job)>,
    sub_selection: usize,
}

impl ControlerWidget {
    pub fn new() -> Self {
        Self{
            sub_selection: 0,
            selected: true,
            sub: vec![
                ("Standby", true, Job::Standby),
                ("EC Monitoring", false, Job::EcMonitor),
                ("PH Monitoring", false, Job::PhMonitor),
                ("PH+EC Monitoring", false, Job::FullMonitor),
            ],
        }
    }
}

impl SelectableWidget for ControlerWidget {
    fn render(&self, _app: &App, frame: &mut Fram, area: Rect) {
        let items: Vec<_> = self.sub.iter().map(|(name, selected, _)| {
            if *selected {
                ListItem::new(format!(">> {}", name)).style( Style::default().fg(Color::Black).bg(Color::White))
            } else {
                ListItem::new(name.to_string()).style( Style::default().fg(Color::White).bg(Color::Black))
            }
        }).collect();
        let items = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Job").border_style(Style::default().fg(if self.selected {Color::White} else {Color::DarkGray}))
            .style(Style::default().bg(Color::Black)));
        frame.render_widget(items, area);
    }

    fn on_key(&mut self, key: Key, app: &mut App) {
        match key {
            Key::Down => {
                self.sub[self.sub_selection].1 = false;
                if self.sub_selection < self.sub.len() - 1 {
                    self.sub_selection += 1;
                } else {
                    self.sub_selection = 0;
                }
                self.sub[self.sub_selection].1 = true;
                app.set_job_kind(self.sub[self.sub_selection].2);
            },
            Key::Up => {
                self.sub[self.sub_selection].1 = false;
                if self.sub_selection > 0  {
                    self.sub_selection -= 1;
                } else {
                    self.sub_selection = self.sub.len() - 1;
                }
                self.sub[self.sub_selection].1 = true;
                app.set_job_kind(self.sub[self.sub_selection].2);
            },
            _ => {},
        }
    }

    fn select(&mut self) {
        self.selected = true;
    }

    fn deselect(&mut self) {
        self.selected = false;
    }
}
