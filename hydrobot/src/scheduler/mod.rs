use actix::prelude::*;
use crate::daemon::*;
use crate::gui::*;
use crate::store::*;
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use std::sync::{Arc, RwLock};
mod utils;
mod tasks;
use tasks::*;
pub use utils::*;

pub type SchedulerResult<T> =Result<T, SchedulerError>;

#[derive(Debug, Fail)]
pub enum SchedulerError {
    #[fail(display = "Board busy: {}", 0)]
    BoardBusy(&'static str),
}

#[derive(Message)]
#[rtype(Result = "()")]
pub enum SchedulerRequest {
    Init {
        handle: SerialDaemonHandle,
        gui: Option<Addr<GuiActor>>,
    },
    Serial {
        result: SerialCommandResult,
        success: bool,
    },
    SetPhMonitorEnabled {
        enabled: bool,
    },
    SetEcMonitorEnabled {
        enabled: bool,
    },
    SetTdsThresh {
        thresh: f64,
    },
    SetOsmoseurPulseDuration {
        duration: std::time::Duration,
    },
    SetOsmoseurPulseMinInterval {
        interval: std::time::Duration,
    },
    SetPhThresh {
        thresh: f64,
    },
    SetPhPulseDuration {
        duration: std::time::Duration,
    },
    SetPhPulseMinInterval {
        interval: std::time::Duration,
    },
}

#[derive(Debug)]
pub struct HardwareError(&'static str);

pub struct PumpHardwareLock {
    locked: bool,
    opened: Option<bool>,
    poisoned: Option<HardwareError>,
}

impl PumpHardwareLock {
    pub fn new() -> Self {
        PumpHardwareLock {
            locked: false,
            opened: None,
            poisoned: None,
        }
    }
}

pub struct SchedulerActor {
    osmoseur_pump: PumpHardwareLock,
    status: Status,
    handle: Option<SerialDaemonHandle>,
    gui: Option<Addr<GuiActor>>,
    store: Store,
    tds_1_samples: SamplesAnalytic,
    tds_monitor: PulseMonitor,
    ph_1_samples: SamplesAnalytic,
    ph_monitor: PulseMonitor,
    ph_monitor_enabled: bool,
    ec_monitor_enabled: bool,
    add_osmosed_water_task: Option<AddOsmoseurWaterTask>,
}

#[derive(Debug, Hash, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Task {
    AddOsmoseurWater,
}

impl SchedulerActor {
    pub fn new(store: Store) -> Self {
        Self {
            ph_monitor_enabled: store.get_ph_monitoring(),
            ec_monitor_enabled: store.get_tds_monitoring(),
            status: Status::NONE,
            handle: None,
            gui: None,
            tds_monitor: PulseMonitor::new(store.get_tds_1_thresh(), store.get_osmoseur_pulse_min_interval(), store.get_osmoseur_pulse_duration()),
            tds_1_samples: SamplesAnalytic::new(20, 4.0, Duration::from_secs(10)),
            ph_monitor: PulseMonitor::new(store.get_ph_1_thresh(), store.get_ph_pulse_min_interval(), store.get_ph_pulse_duration()),
            ph_1_samples: SamplesAnalytic::new(20, 0.1, Duration::from_secs(10)),
            store,
            osmoseur_pump: PumpHardwareLock::new(),
            add_osmosed_water_task: None,
        }
    }

    fn to_board(&mut self, req: SerialCommand) {
        self.handle.as_mut().unwrap().send(req).expect("Serial port");
    }

    fn info<T: ToString>(&self, msg: T) {
        if let Some(gui) = self.gui.as_ref() {
            gui.do_send(GuiEvent::Log(SystemTime::now(), msg.to_string(), LogLevel::Info))
        }
    }

    fn query<T: ToString>(&self, msg: T) {
        if let Some(gui) = self.gui.as_ref() {
            gui.do_send(GuiEvent::Query(SystemTime::now(), msg.to_string()))
        }
    }

    fn warn<T: ToString>(&self, msg: T) {
        if let Some(gui) = self.gui.as_ref() {
            gui.do_send(GuiEvent::Log(SystemTime::now(), msg.to_string(), LogLevel::Warn))
        }
    }

    fn error<T: ToString>(&self, msg: T) {
        if let Some(gui) = self.gui.as_ref() {
            gui.do_send(GuiEvent::Log(SystemTime::now(), msg.to_string(), LogLevel::Error))
        }
    }

    fn to_gui(&self, req: GuiEvent) {
        if let Some(gui) = self.gui.as_ref() {
            gui.do_send(req);
        }
    }
}

impl Handler<SchedulerRequest> for SchedulerActor {
    type Result = ();
    fn handle(&mut self, msg: SchedulerRequest, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            SchedulerRequest::SetTdsThresh {  thresh } => {
                self.info(format!("Tds threshold updated to {}", thresh));
                self.store.set_tds_1_thresh(thresh);
                self.tds_monitor.threshold = thresh;
            }
            SchedulerRequest::SetOsmoseurPulseDuration {  duration } => {
                self.info(format!("Osmoseur pulse duration updated to {}", duration.as_secs()));
                self.store.set_osmoseur_pulse_duration(duration);
                self.tds_monitor.pulse_duration = duration;
            }
            SchedulerRequest::SetOsmoseurPulseMinInterval {  interval } => {
                self.info(format!("Osmoseur pulse minimum interval updated to {}", interval.as_secs()));
                self.store.set_osmoseur_pulse_min_interval(interval);
                self.tds_monitor.pulse_minimum_interval = interval;
            }
            SchedulerRequest::SetPhThresh {  thresh } => {
                self.info(format!("ph threshold updated to {}", thresh));
                self.store.set_ph_1_thresh(thresh);
                self.ph_monitor.threshold = thresh;
            }
            SchedulerRequest::SetPhPulseDuration {  duration } => {
                self.info(format!("ph pulse duration updated to {}", duration.as_secs()));
                self.store.set_ph_pulse_duration(duration);
                self.ph_monitor.pulse_duration = duration;
            }
            SchedulerRequest::SetPhPulseMinInterval {  interval } => {
                self.info(format!("ph pulse minimum interval updated to {}", interval.as_secs()));
                self.store.set_ph_pulse_min_interval(interval);
                self.ph_monitor.pulse_minimum_interval = interval;
            }
            SchedulerRequest::SetEcMonitorEnabled { enabled } => {
                self.ec_monitor_enabled = enabled;
                self.store.set_tds_monitoring(enabled);
            },
            SchedulerRequest::SetPhMonitorEnabled { enabled } => {
                self.ph_monitor_enabled = enabled;
                self.store.set_ph_monitoring(enabled);
            },
            SchedulerRequest::Init { handle , gui} => {
                self.handle = Some(handle);
                self.gui = gui;
                ctx.run_interval(Duration::from_secs(1), |actor: &mut Self, _| {
                    actor.to_board(SerialCommand::G1);
                });
            },
            SchedulerRequest::Serial { result, success } => {
                // self.info(format!("Recv ({}) {:?}",if success {"OK"} else{"ERROR"}, &result));
                match result {
                    SerialCommandResult::S0 { on } if success => { self.osmoseur_pump.opened = on; },
                    SerialCommandResult::S0 { .. } => { self.osmoseur_pump.poisoned = Some(HardwareError("Osmoseur pump healted")); },
                    SerialCommandResult::S1 { .. } => {
                        // self.listeners.get_mut("S1").map(|m| m.replace(e));
                    },
                    SerialCommandResult::G0 {..} => {},
                    SerialCommandResult::G1 { tds_1, ph_1, status, t_1 } => {
                        if let Some(status) = status {
                            if status.contains(Status::TDS_CONNECTED) && !self.status.contains(Status::TDS_CONNECTED) {
                                self.info("TDS probe connected !");
                                self.tds_1_samples.clear();
                            }
                            else if !status.contains(Status::TDS_CONNECTED) && self.status.contains(Status::TDS_CONNECTED) {
                                self.warn("TDS probe disconnected !");
                            }
                            if status.contains(Status::PH_CONNECTED) && !self.status.contains(Status::PH_CONNECTED) {
                                self.info("PH probe donnected !");
                                self.tds_1_samples.clear();
                            }
                            else if !status.contains(Status::PH_CONNECTED) && self.status.contains(Status::PH_CONNECTED) {
                                self.warn("PH probe disconnected !");
                            }
                            self.status = status;
                            self.to_gui(GuiEvent::Status(status));
                        }
                        if self.status.contains(Status::TDS_CONNECTED) {
                            if let Some(sample) = tds_1 {
                                self.to_gui(GuiEvent::TdsSensore(sample, self.tds_1_samples.status));
                                if let AnalyticStatus::Stable(current) = self.tds_1_samples.status {
                                    if self.ec_monitor_enabled {
                                        if let Some(duration) = self.tds_monitor.update(current) {
                                            if self.add_osmosed_water_task.is_some() {
                                                self.query("Can't lower TDS for now, the task is already pending !");
                                            } else { 
                                                self.add_osmosed_water_task = Some(AddOsmoseurWaterTask::new(duration));
                                                self.query("Lowering TDS value (adding clean water)");
                                            }
                                        }
                                    }
                                } 
                            }
                        }
                        
                        if self.status.contains(Status::PH_CONNECTED) {
                            if let Some(sample) = ph_1 {
                                self.to_gui(GuiEvent::PhSensore(sample, self.ph_1_samples.status));
                                if let AnalyticStatus::Stable(current) = self.ph_1_samples.status {
                                    if self.ph_monitor_enabled {
                                        if let Some(duration) = self.tds_monitor.update(current) {
                                            if self.add_osmosed_water_task.is_some() {
                                                self.query("Can't lower PH for now, the task is already pending !");
                                            } else { 
                                                self.add_osmosed_water_task = Some(AddOsmoseurWaterTask::new(duration));
                                                self.query("Lowering PH value (adding PH Down water)");
                                            }
                                        }
                                    }
                                } 
                            }
                        }

                        if let Some(sample) = t_1 {
                            self.to_gui(GuiEvent::TemperatureSensore(sample));
                        }
                    },
                    SerialCommandResult::Unknown{raw} => {
                        self.info(format!("Unknown command result: `{}`", raw));
                    }
            }
            }
        }
    }
}

impl Actor for SchedulerActor {
    type Context = Context<SchedulerActor>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(std::time::Duration::from_millis(200), |actor, ctx|{
            if let Some(task) = actor.add_osmosed_water_task.take() {
                actor.update_add_osmosed_water_task(task, ctx);
            }
        });
    }
}
