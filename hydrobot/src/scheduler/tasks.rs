use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::time::SystemTime;
use std::future::Future;
use std::task::{Context, Poll, Waker};
use std::pin::Pin;
use actix::{Context as ActorContext, ActorFuture};
use super::*;

#[derive(Clone, Copy, Debug)]
enum AddOsmoseurWaterStatus {
    WaitLock,
    WaitOpen,
    WaitClose,
    WaitDuration,
}

pub struct AddOsmoseurWaterTask {
    status: AddOsmoseurWaterStatus,
    duration: Duration,
    begin: Option<SystemTime>,
}

impl AddOsmoseurWaterTask {
    pub fn new(duration: Duration) -> Self {
        Self {
            status: AddOsmoseurWaterStatus::WaitLock,
            begin: None,
            duration: duration,
        }
    }
}

impl SchedulerActor {
    pub fn update_add_osmosed_water_task(&mut self, mut task: AddOsmoseurWaterTask, cx: &mut ActorContext<SchedulerActor>) {
        match task.status {
            AddOsmoseurWaterStatus::WaitLock if !self.osmoseur_pump.locked => {
                self.osmoseur_pump.locked = true;
                self.osmoseur_pump.opened = None;
                self.handle.as_mut().unwrap().send(SerialCommand::S0{ on: true }).unwrap();
                task.status = AddOsmoseurWaterStatus::WaitOpen;
            },
            AddOsmoseurWaterStatus::WaitOpen => {
                match self.osmoseur_pump.opened.as_ref() {
                    Some(true) => {
                        self.info("Osmoseur valve opened !");
                        task.begin.replace(SystemTime::now());
                        task.status = AddOsmoseurWaterStatus::WaitDuration;
                    },
                    Some(false) => {
                        self.error("Failed to open valve !");
                        return;
                    },
                    _ => self.info("Wait osmoseur valve to be opened ..."),
                }
            }
            AddOsmoseurWaterStatus::WaitDuration if task.begin.as_ref().unwrap().elapsed().unwrap() >= task.duration => {
                self.handle.as_mut().unwrap().send(SerialCommand::S0{on: false}).expect("Board disconnected !");
                self.info("Wait osmoseur valve to be closed ...");
                task.status = AddOsmoseurWaterStatus::WaitClose;
            },
            AddOsmoseurWaterStatus::WaitClose if !self.osmoseur_pump.opened.unwrap_or_default() => {
                self.info("Osmoseur valve closed !");
                self.tds_monitor.resume();
                self.osmoseur_pump.locked = false;
                return;
            },
            _ => {},
        }
        self.add_osmosed_water_task = Some(task);
    }
}