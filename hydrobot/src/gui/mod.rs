
use crate::daemon::Status;
use actix::prelude::*;
use std::{
    collections::{VecDeque},
    error::Error, io,io::Stdout};
use termion::{raw::RawTerminal, event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Rect, Constraint, Direction, Layout},
    Terminal,
    Frame,
};
use std::time::SystemTime;
use crate::scheduler::*;
use crate::store::Store;
use termion::input::TermRead;

mod widgets;
use widgets::*;


const MAX_TDS_SAMPLES: usize = 256;
const MAX_PH_SAMPLES: usize = 256;
const MAX_LOG: usize = 256;

pub enum LogLevel {
    Info,
    Error,
    Warn,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        })
    }
}

#[derive(Message)]
#[rtype(Result = "()")]
pub enum GuiEvent {
    Key(Key),
    Log(SystemTime, String, LogLevel),
    Query(SystemTime, String),
    TdsSensore(f64, AnalyticStatus),
    PhSensore(f64, AnalyticStatus),
    Status(Status),
}

type Term = Terminal<TermionBackend<AlternateScreen<MouseTerminal<RawTerminal<Stdout>>>>>;
type Fram<'a> = Frame<'a, TermionBackend<AlternateScreen<MouseTerminal<RawTerminal<Stdout>>>>>;

pub struct GuiActor {
    widgets: Vec<Box<dyn SelectableWidget>>,
    current_selection: usize,
    terminal: Term,
    app: App,
}

pub struct App {
    selected_setting_categorie: SettingCategorie,
    focused: bool,
    scheduler: Addr<SchedulerActor>,
    status: Status,
    store: Store,
    tds: f64,
    tds_status: AnalyticStatus,
    ph_status: AnalyticStatus,
    tds_buffer_trunc: Vec<(f64, f64)>,
    ph: f64,
    ph_buffer_trunc: Vec<(f64, f64)>,
    logs: VecDeque<(SystemTime, String, LogLevel)>,
    queries: VecDeque<(SystemTime, String)>,
}

pub trait SelectableWidget {
    fn render(&self, app: &App, frame: &mut Fram, area: Rect);
    fn select(&mut self);
    fn deselect(&mut self);
    fn on_key(&mut self, _key: Key, _app: &mut App) {
    }
}
impl App {

    fn draw(&mut self, terminal: &mut Term, widgets: &[Box<dyn SelectableWidget>]) -> Result<(), Box<dyn Error>>  {
        terminal.draw(|f| {
            // Init main linear layout
            let vchunk = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Percentage(20),
                        Constraint::Percentage(20),
                        Constraint::Percentage(20),
                        Constraint::Percentage(60),
                    ]
                    .as_ref(),
                )
                .split(f.size());
    
            widgets[0].render(&self, f, vchunk[0]);//tds
            widgets[1].render(&self, f, vchunk[1]);//ph

            let control_column = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(20),
                    Constraint::Percentage(80),
                ]
                .as_ref()).split(vchunk[2]);
            widgets[2].render(&self, f, control_column[0]);//control
            widgets[3].render(&self, f, control_column[1]);//control details
            let logs_column = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ].as_ref()).split(vchunk[3]);
            widgets[4].render(&self, f, logs_column[0]);//feedback
            widgets[5].render(&self, f, logs_column[1]);//log
        })?;
        Ok(())
    }
}


impl GuiActor {

    fn select_next(&mut self) {
        self.widgets[self.current_selection].deselect();
        if self.current_selection < self.widgets.len() - 1 {
            self.current_selection += 1;
        } else {
            self.current_selection = 0;
        }
        self.widgets[self.current_selection].select();
    }

    fn select_prev(&mut self) {
        self.widgets[self.current_selection].deselect();
        if self.current_selection > 0 {
            self.current_selection -= 1;
        } else {
            self.current_selection = self.widgets.len() - 1;
        }
        self.widgets[self.current_selection].select();
    }

    pub fn new(scheduler: Addr<SchedulerActor>, store: Store) -> Self{
        // Terminal initialization
        let stdout = io::stdout().into_raw_mode().expect("Failed to init get stdout raw");
        let stdout = MouseTerminal::from(stdout);
        let stdout = AlternateScreen::from(stdout);
        let backend = TermionBackend::new(stdout);
        let terminal = Terminal::new(backend).expect("Failed to get term handle");
        Self {
            widgets: vec![
                Box::from(TdsWidget::new()),
                Box::from(PhWidget::new()),
                Box::from(ControlerWidget::new()),
                Box::from(ControlerDetailsWidget::new(&store)),
                Box::from(FeedBackWidget::new()),
                Box::from(QueryWidget::new()),
            ],
            current_selection: 2,
            terminal,
            app: App {
                selected_setting_categorie: SettingCategorie::General,
                focused: false,
                scheduler,
                status: Status::NONE,
                tds: 0.0,
                tds_status: AnalyticStatus::Undefined,
                ph_status: AnalyticStatus::Undefined,
                ph: 0.0,
                store: store,
                logs: VecDeque::new(),
                queries: VecDeque::new(),
                tds_buffer_trunc: Vec::with_capacity(MAX_TDS_SAMPLES),
                ph_buffer_trunc: Vec::with_capacity(MAX_PH_SAMPLES),
            }
        }
    }
}

impl Handler<GuiEvent> for GuiActor {
    type Result = ();
    
    fn handle(&mut self, msg: GuiEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            GuiEvent::Status(status) => {
                self.app.status = status;
            },
            GuiEvent::TdsSensore(tds, status) => {
                self.app.tds_buffer_trunc.push((std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as f64, tds));
                if self.app.tds_buffer_trunc.len() > MAX_TDS_SAMPLES {
                    self.app.tds_buffer_trunc.remove(0);
                }
                self.app.tds = tds;
                self.app.tds_status = status;
            },
            GuiEvent::PhSensore(ph, status) => {
                self.app.ph_buffer_trunc.push((std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as f64, ph));
                if self.app.ph_buffer_trunc.len() > MAX_PH_SAMPLES {
                    self.app.ph_buffer_trunc.remove(0);
                }
                self.app.ph = ph;
                self.app.ph_status = status;

            },
            GuiEvent::Log(date, msg, level) => {
                self.app.logs.push_back((date, msg, level));
                if self.app.logs.len() > MAX_LOG {
                    self.app.logs.pop_front();
                }
            }
            GuiEvent::Query(date, msg ) => {
                self.app.queries.push_back((date, msg));
                if self.app.queries.len() > MAX_LOG {
                    self.app.queries.pop_front();
                }
            }
            GuiEvent::Key(key) => match key {
                Key::Char('q') => {
                    ctx.stop();
                    System::current().stop();
                }
                Key::Left if !self.app.focused => {
                    self.select_prev();
                },
                Key::Right if !self.app.focused  => {
                    self.select_next();
                },
                x => self.widgets[self.current_selection].on_key(x, &mut self.app),
            }
        }
    }
}

impl Actor for  GuiActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(std::time::Duration::from_millis(200), |actor: &mut Self, _| {
            let _ = actor.app.draw(&mut actor.terminal, &actor.widgets);
        });
        let addr = ctx.address();
        std::thread::spawn(move || {
            let stdin = io::stdin();
            for evt in stdin.keys() {
                if let Ok(key) = evt {
                    addr.do_send(GuiEvent::Key(key))
                }
            }
        });
    }
}