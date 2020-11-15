use tui::{
    layout::{Rect},
    style::{Color, Style, Modifier},
    widgets::{Block, Borders},
    widgets::{Axis,  Chart, Dataset },
    symbols,
    text::{Span},
};
use super::super::*;

pub struct PhWidget {
    selected: bool,
}

impl PhWidget {
    pub fn new() -> Self {
        Self {
            selected: false,
        }
    }
}

impl SelectableWidget for PhWidget {

    fn render(& self, _app: &App, frame: &mut Fram, area: Rect) {
        let datasets = vec![
            Dataset::default()
                .name("PH 1")
                .marker(symbols::Marker::Dot)
                .data(&[]),
        ];
        let x_labels =  vec![
            Span::raw(""),
            Span::styled(
                "Probe not connected !",
                Style::default().fg(Color::Red)
            ),
        ];
        let min = &(0.0, 0.0);
        let max = &(0.0, 0.0);
        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title(Span::styled(
                        "PH 1",
                        Style::default()
                            .fg(Color::Cyan)
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
                        Span::styled(format!("PH {}", min.1), Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(format!("PH {}", max.1), Style::default().add_modifier(Modifier::BOLD)),
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