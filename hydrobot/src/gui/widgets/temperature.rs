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

pub struct TemperatureWidget {
    selected: bool,
}

impl TemperatureWidget {
    pub fn new() -> Self {
        Self {
            selected: false,
        }
    }
}

impl SelectableWidget for TemperatureWidget {

    fn render(& self, app: &App, frame: &mut Fram, area: Rect) {
        let datasets = vec![
            Dataset::default()
                .name("Temperature")
                .marker(symbols::Marker::Dot)
               
                .data(&app.temperature_buffer_trunc),
        ];
        let x_labels = vec![
                Span::raw("Current : "),
                Span::styled(
                    format!("{}°", app.temperature),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
        ];
        let time_min = app.temperature_buffer_trunc.first().map(|(e, _)| *e).unwrap_or(0.0);
        let time_max = app.temperature_buffer_trunc.last().map(|(e, _)| *e).unwrap_or(0.0);
        let val_max = app.temperature_buffer_trunc.iter().map(|(_, v)| (v * 100.0).round() as u64).max().unwrap_or(0) as f64 / 100.0;
        let val_min = app.temperature_buffer_trunc.iter().map(|(_, v)| (v * 100.0).round() as u64).min().unwrap_or(0) as f64 / 100.0;
        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title(Span::styled(
                        format!("Temperature"),
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
                        Span::styled(format!("{}°", val_min), Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(format!("{}°", val_max), Style::default().add_modifier(Modifier::BOLD)),
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

