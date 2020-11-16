
use termion::{event::Key};
use tui::{
    layout::{Rect},
    style::{Color, Style},
    widgets::{Block, Borders},
    widgets::{List, ListItem },
};
use std::time::Duration;
use std::collections::HashMap;
use super::super::*;

#[derive(Debug, Clone, Copy)]
pub enum ParamKind {
    Boolean(bool),
    Float(f64),
    Int(i64),
    Duration(Duration),
}

impl ParamKind {
    pub fn float_mut(&mut self) -> &mut f64 {
        if let ParamKind::Float(f) = self {
            f
        } else {
            panic!("float_mut called on a non f64 value !")
        }
    }

    pub fn float(self) -> f64 {
        if let ParamKind::Float(f) = self {
            f
        } else {
            panic!("float_mut called on a non f64 value !")
        }
    }

    pub fn duration(self) -> Duration {
        if let ParamKind::Duration(f) = self {
            f
        } else {
            panic!("duration called on a non Duration value !")
        }
    }

    pub fn bool(self) -> bool {
        if let ParamKind::Boolean(f) = self {
            f
        } else {
            panic!("unwrap_bool called on a non bool value !")
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ParamStatus {
    None,
    Selected,
    Editing,
}

impl ParamStatus {
    fn is_editing(&self) -> bool {
        if let ParamStatus::Editing = self {
            true
        } else {
            false
        }
    }
}

pub struct ParamWidget {
    status: ParamStatus,
    name: String,
    can_edit: bool,
    postfix: Option<String>,
    prefix: Option<String>,
    kind: ParamKind,
    apply_ref: Option<Box<dyn (FnMut(&mut ParamKind, &App))>>,
    apply_val: Option<Box<dyn (FnMut(&ParamKind, &mut App))>>,
}

impl ParamWidget {
    fn new<T: ToString>(name: T,  kind: ParamKind) -> Self {
        Self {
            name: name.to_string(),
            kind: kind,
            can_edit: false,
            postfix: None,
            prefix: None,
            status: ParamStatus::None,
            apply_ref: None,
            apply_val: None,
        }
    }

    fn apply_ref(mut self, f: Box<dyn (FnMut(&mut ParamKind, &App))>) -> Self {
        self.apply_ref = Some(f);
        self
    }

    fn apply_val(mut self, f: Box<dyn (FnMut(&ParamKind, &mut App))>) -> Self {
        self.apply_val = Some(f);
        self
    }

    fn can_edit(mut self, edit: bool) -> Self {
        self.can_edit = edit;
        self
    }

    fn postfix<T: ToString>(mut self, postfix: Option<T>) -> Self {
        self.postfix = postfix.map(|e| e.to_string());
        self
    }

    fn prefix<T: ToString>(mut self, prefix: Option<T>) -> Self {
        self.prefix = prefix.map(|e| e.to_string());
        self
    }
}

#[derive(Debug,Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SettingCategorie {
    General,
    EcMonitor,
    PhMonitor,
}

pub struct ControlerDetailsWidget {
    selected:bool,
    widgets: HashMap<SettingCategorie, Vec<ParamWidget>>,
}

impl ControlerDetailsWidget {

    pub fn new(store: &Store) -> Self {
        let mut widgets = HashMap::new();
        widgets.insert(SettingCategorie::General, vec![
            ParamWidget::new("EC Compensation", ParamKind::Boolean(store.get_tds_monitoring()))
                .can_edit(true)
                .apply_val(Box::from(|kind: &ParamKind, app: &mut App| {
                    app.scheduler.do_send(SchedulerRequest::SetEcMonitorEnabled { enabled: kind.bool() });
                }))
            ,
            ParamWidget::new("PH Compensation", ParamKind::Boolean(store.get_ph_monitoring()))
                .can_edit(true)
                .apply_val(Box::from(|kind: &ParamKind, app: &mut App| {
                    app.scheduler.do_send(SchedulerRequest::SetPhMonitorEnabled { enabled: kind.bool() });
                }))
        ]);
        widgets.insert(SettingCategorie::EcMonitor, vec![
            ParamWidget::new("Threshold", ParamKind::Float(store.get_tds_1_thresh()))
                .postfix(Some("PPM"))
                .can_edit(true)
                .apply_ref(Box::from(|kind: &mut ParamKind, app: &App| { *kind.float_mut() = app.tds; }))
                .apply_val(Box::from(|kind: &ParamKind, app: &mut App| {
                    app.scheduler.do_send(SchedulerRequest::SetTdsThresh { thresh: kind.float() });
                })
            ),
            ParamWidget::new("Osmoseur pulse duration", ParamKind::Duration(store.get_osmoseur_pulse_duration()))
                .can_edit(true)
                .apply_val(Box::from(|kind: &ParamKind, app: &mut App| {
                   app.scheduler.do_send(SchedulerRequest::SetOsmoseurPulseDuration { duration: kind.duration() });
                })
            ),
            ParamWidget::new("Osmoseur pulse interval", ParamKind::Duration(store.get_osmoseur_pulse_min_interval()))
                .can_edit(true)
                .apply_val(Box::from(|kind: &ParamKind, app: &mut App| {
                   app.scheduler.do_send(SchedulerRequest::SetOsmoseurPulseMinInterval { interval: kind.duration() });
                })
            ),
            ParamWidget::new("Total water added", ParamKind::Int(0)).postfix(Some("ML")),
        ]);
        widgets.insert(SettingCategorie::PhMonitor, vec![
            ParamWidget::new("Threshold", ParamKind::Float(store.get_ph_1_thresh()))
                .prefix(Some("PH"))
                .can_edit(true)
                .apply_ref(Box::from(|kind: &mut ParamKind, app: &App| { *kind.float_mut() = app.ph; }))
                .apply_val(Box::from(|kind: &ParamKind, app: &mut App| {
                        app.scheduler.do_send(SchedulerRequest::SetPhThresh { thresh: kind.float() });
                })
            ),
            ParamWidget::new("PH Down pulse duration", ParamKind::Duration(store.get_ph_pulse_duration()))
                .can_edit(true)
                .apply_val(Box::from(|kind: &ParamKind, app: &mut App| {
                   app.scheduler.do_send(SchedulerRequest::SetPhPulseDuration { duration: kind.duration() });
                })
            ),
            ParamWidget::new("PH Down pulse interval", ParamKind::Duration(store.get_ph_pulse_min_interval()))
                .can_edit(true)
                .apply_val(Box::from(|kind: &ParamKind, app: &mut App| {
                   app.scheduler.do_send(SchedulerRequest::SetPhPulseMinInterval { interval: kind.duration() });
                })
            ),
            ParamWidget::new("Total PH Down added", ParamKind::Int(0)).postfix(Some("ML")),
        ]);
        Self{
            widgets,
            selected: true,
        }
    }
}

impl SelectableWidget for ControlerDetailsWidget {
    fn render(&self, _app: &App, frame: &mut Fram, area: Rect) {
        let items: Vec<ListItem> = self.widgets[&_app.selected_setting_categorie].iter().map(|e| {
            let item = ListItem::new(e.name.clone());
            item
        }).collect();

        let values: Vec<ListItem> = self.widgets[&_app.selected_setting_categorie].iter().map(|e| {
            let value = match e.kind {
                ParamKind::Boolean(e) => format!("{}", e),
                ParamKind::Duration(e) => format!("{:?}", e),
                ParamKind::Float(e) => format!("{}", e),
                ParamKind::Int(e) => format!("{}", e),
            };
            let value_str = match () {
                _ if e.prefix.is_some() && e.postfix.is_some() => format!("{}{}{}",e.prefix.as_ref().unwrap(), value, e.postfix.as_ref().unwrap()),
                _ if e.prefix.is_some() => format!("{}{}", e.prefix.as_ref().unwrap(), value),
                _ if e.postfix.is_some() => format!("{}{}", value, e.postfix.as_ref().unwrap()),
                _ => format!("{}", value),
            };
            match e.status {
                ParamStatus::None => ListItem::new(value_str),
                ParamStatus::Selected => ListItem::new(format!("{}", value_str)).style(Style::default().fg(if e.can_edit { Color::White} else { Color::DarkGray })),
                ParamStatus::Editing => ListItem::new(format!("[{}]", value_str)).style(Style::default().fg(Color::White)),
            }
        }).collect();

        let control_column = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(80),
        ]
        .as_ref()).split(area);
        let items = List::new(items)
        .block(Block::default().borders(Borders::TOP | Borders::BOTTOM).border_style(Style::default().fg(if self.selected {Color::White} else {Color::DarkGray})));

        frame.render_widget(items, control_column[0]);
        let value = List::new(values)
        .block(Block::default().borders(Borders::TOP | Borders::BOTTOM | Borders::RIGHT).border_style(Style::default().fg(if self.selected {Color::White} else {Color::DarkGray})));
        

        frame.render_widget(value, control_column[1]);
    }

    fn on_key(&mut self, key: Key, app: &mut App) {
        let current_list = self.widgets.get_mut(&app.selected_setting_categorie).unwrap();
        let current_selection = current_list.iter_mut().enumerate().find(|(_, e)| e.status != ParamStatus::None);
        match (key, current_selection) {
            (Key::Insert, Some((_, selection))) if selection.can_edit => selection.status = match selection.status {
                ParamStatus::Editing => {
                    app.focused = false;
                    if let Some(apply) = selection.apply_val.as_mut() {
                        apply(&mut selection.kind, app);
                    }
                    ParamStatus::Selected
                },
                _ => {
                    app.focused = true;
                    ParamStatus::Editing
                },
            },
            (Key::Down, Some((idx, selection))) if !selection.status.is_editing() => {
                selection.status = ParamStatus::None;
                if idx < current_list.len() - 1 {
                    current_list[idx + 1].status = ParamStatus::Selected;
                } else {
                    current_list[0].status = ParamStatus::Selected;
                }
            }
            (Key::Up, Some((idx, selection))) if !selection.status.is_editing() => {
                selection.status = ParamStatus::None;
                if idx > 0 {
                    current_list[idx - 1].status = ParamStatus::Selected;
                } else {
                    current_list.last_mut().unwrap().status = ParamStatus::Selected;
                }
            },
            (Key::Down, Some((_idx, selection))) => match selection.kind {
                ParamKind::Boolean(ref mut value) => *value = !*value,
                ParamKind::Float(ref mut value) => *value = *value - 1.0,
                ParamKind::Duration(ref mut value) => *value = Duration::from_secs(value.as_secs() - 1),
                _ => {},
            },
            (Key::Up, Some((_idx, selection))) => match selection.kind {
                ParamKind::Boolean(ref mut value) => *value = !*value,
                ParamKind::Float(ref mut value) => *value = *value + 1.0,
                ParamKind::Duration(ref mut value) => *value = Duration::from_secs(value.as_secs() + 1),
                _ => {},
            },
            (Key::Char('r'), Some((_idx, selection))) if selection.status.is_editing() && selection.apply_ref.is_some() => selection.apply_ref.as_mut().unwrap()(&mut selection.kind, app),
            (Key::Up, None) => current_list[0].status = ParamStatus::Selected,
            (Key::Down, None) => current_list[0].status = ParamStatus::Selected,
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
