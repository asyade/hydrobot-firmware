use crate::daemon::Status;
use tui::{
    layout::{Rect},
    style::{Color, Style, Modifier},
    widgets::{Block, Borders},
    widgets::{Axis,  Chart, Dataset },
    symbols,
    text::{Span},
};
use super::super::*;

pub struct TdsWidget {
    selected: bool,
}

impl TdsWidget {
    pub fn new() -> Self {
        Self {
            selected: false,
        }
    }
}

impl SelectableWidget for TdsWidget {

    fn render(& self, app: &App, frame: &mut Fram, area: Rect) {
        let datasets = vec![
            Dataset::default()
                .name("TDS")
                .marker(symbols::Marker::Dot)
               
                .data(&app.tds_buffer_trunc),
        ];
        let postfix = match app.tds_status {
            AnalyticStatus::Uprising(_) => "PPM ⇑",
            AnalyticStatus::Downrising(_) => "PPM ⇓",
            AnalyticStatus::Stable(_) => "PPM ⍻",
            AnalyticStatus::Stabilizing(_,_) => "PPM ⏳",
            _ => "PPM ?"
        };
        let x_labels = if app.status.contains(Status::TDS_CONNECTED) {
            vec![
                Span::raw("Current : "),
                Span::styled(
                    format!("{} {}", app.tds, postfix),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("{}", "Target : ")),
                Span::styled(
                    format!("TDS {}", app.store.get_tds_1_thresh()),
                    Style::default().add_modifier(Modifier::BOLD).bg(if self.selected { Color::White} else { Color:: Black })
                ),
            ]
        } else {
            vec![
                Span::raw(""),
                Span::styled(
                    "Probe not connected !",
                    Style::default().fg(Color::Red)
                ),
            ]
        };
        let time_min = app.tds_buffer_trunc.first().map(|(e, _)| *e).unwrap_or(0.0);
        let time_max = app.tds_buffer_trunc.last().map(|(e, _)| *e).unwrap_or(0.0);
        let val_max = app.tds_buffer_trunc.iter().map(|(_, v)| (v * 100.0).round() as u64).max().unwrap_or(0) as f64 / 100.0;
        let val_min = app.tds_buffer_trunc.iter().map(|(_, v)| (v * 100.0).round() as u64).min().unwrap_or(0) as f64 / 100.0;
        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title(Span::styled(
                        format!("TDS"),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(if self.selected {Color::White} else {Color::DarkGray})),
            )
            .x_axis(
                Axis::default()
                    .title("")
                    .style(Style::default().fg(Color::Gray))
                    .labels(x_labels)
                    .bounds([time_min, time_max])
            )
            .y_axis(
                Axis::default()
                    .title("")
                    .style(Style::default().fg(Color::Gray))
                    .labels(vec![
                        Span::styled(format!("{} PPM", val_min), Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(format!("{} PPM", val_max), Style::default().add_modifier(Modifier::BOLD)),
                    ])
                    .bounds([val_min, val_max])
            );
        frame.render_widget(chart, area)
    }

    fn select(&mut self) {
        self.selected = true;
    }
    
    fn deselect(&mut self) {
        self.selected = false;
    }
}

