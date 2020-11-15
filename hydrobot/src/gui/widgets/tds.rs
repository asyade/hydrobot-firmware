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
                .name("TDS 1")
                .marker(symbols::Marker::Dot)
                .data(&app.tds_1_buffer_trunc),
        ];
        let x_labels = if app.status.contains(Status::TDS_1_CONNECTED) {
            vec![
                Span::raw("Current : "),
                Span::styled(
                    format!("{}PPM", app.tds_1),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("{}", "Target : ")),
                Span::styled(
                    format!("<>"),
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
        let min = app.tds_1_buffer_trunc.first().unwrap_or(&(0.0, 0.0));
        let max = app.tds_1_buffer_trunc.last().unwrap_or(&(0.0, 0.0));
        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title(Span::styled(
                        "TDS 1",
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
                    .bounds([min.0, max.0])
            )
            .y_axis(
                Axis::default()
                    .title("")
                    .style(Style::default().fg(Color::Gray))
                    .labels(vec![
                        Span::styled(format!("{}PPM", min.1), Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(format!("{}PPM", max.1), Style::default().add_modifier(Modifier::BOLD)),
                    ])
                    .bounds([min.1, max.1])
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

