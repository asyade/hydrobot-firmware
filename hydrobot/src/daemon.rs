use actix::prelude::*;
use serialport::{SerialPort};
use std::thread;
use std::io::{Write, BufRead, BufReader};
use std::{fmt, fmt::{Formatter, Display}};
use crate::scheduler::*;
pub struct SerialDaemon {
    reader: BufReader<Box<dyn SerialPort>>,
    sceduler: Addr<SchedulerActor>,
}

bitflags! {
    pub struct Status: u32 {
        const NONE                          = 0;
        const TDS_CONNECTED                 = ((1 << 0));
        const PH_CONNECTED                  = ((1 << 2));
        const TEMPERATURE_CONNECTED         = ((1 << 3));
        const OSMOS_SWITCH_OPENED           = ((1 << 4));
        const OSMOS_SWITCH_OPENING          = ((1 << 5));
        const OSMOS_SWITCH_CLOSING          = ((1 << 6));
        const OSMOS_SWITCH_CLOSED           = ((1 << 7));
        const PERISTALIC_PUMP_ON            = ((1 << 8));
        const PERISTALIC_PUMP_REV           = ((1 << 9));
        const BRONCHUS_STANDBY_FULL         = ((1 << 10));
        const BRONCHUS_STANDBY_SAMPLING     = ((1 << 11));
        const BRONCHUS_WAIT_FULL            = ((1 << 12));
        const BRONCHUS_WAIT_EMPTY           = ((1 << 13));
        const BREATHING                     = ((1 << 14));
    }
}

impl SerialDaemon {
    pub fn new(port: Box<dyn SerialPort>, sceduler: Addr<SchedulerActor>) -> SerialDaemonHandle {
        let tty = port.try_clone().expect("Duplex not usported on the tty");
        let read_loop = thread::spawn(move || {
            SerialDaemon {
                reader: BufReader::new(tty),
                sceduler,
            }.run()
        });
        SerialDaemonHandle {
            _read_loop: read_loop,
            port,
        }
    }

    fn run(&mut self) {
        loop {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(_) => {
                    match SerialCommandResult::from_string(&line) {
                        Some((result, success)) => {
                            self.sceduler.do_send(SchedulerRequest::Serial {result, success});
                        },
                        None => warn!("Failed to parse `{:?}`", line)
                    }
                }
                Err(e) => {
                    error!("Board disconnected: {:?}", e);
                    break;
                }
            }
        }
        warn!("Serial port disconnected !");
    }
}

pub struct SerialDaemonHandle {
    port: Box<dyn SerialPort>,
    _read_loop: thread::JoinHandle<()>,
}

#[derive(Debug, Clone)]
pub enum SerialCommandResult {
    G0 {
        tds_1: Option<f64>,
        ph_1: Option<f64>,
    },
    G1 {
        tds_1: Option<f64>,
        ph_1: Option<f64>,
        t_1: Option<f64>,
        status: Option<Status>,
    },
    S0 {
        on: Option<bool>,
    },
    S1 {
        on: Option<bool>,
    },
    Unknown {
        raw: String,
    }
}

impl SerialCommandResult {
    fn from_string(val: &str) -> Option<(SerialCommandResult, bool)> {
        info!("Receive {:?}", val);
        let mut parts =val.split(" ").into_iter().map(|e| e.trim().to_uppercase());
        let success = parts.next()? == "OK";
        match parts.next()?.as_str() {
            "S0" => {
                let on: Option<bool> = parts.next().map(|e| e.trim().eq("ON"));
                Some((SerialCommandResult::S0 { on }, success))
            },
            "S1" => {
                let on: Option<bool> = parts.next().map(|e| e.trim().eq("ON"));
                Some((SerialCommandResult::S1 { on }, success))
            },
            "G1" => {
                let mut tds_1: Option<f64> = None;
                let mut t_1: Option<f64> = None;
                let mut ph_1: Option<f64> = None;
                let mut status: Option<Status> = None;
                while let Some(part) = parts.next() {
                    match part.as_str() {
                        "TDS1" => {
                            tds_1 = Some(parts.next()?.parse().ok()?);
                        },
                        "PH1" => {
                            ph_1 = Some(parts.next()?.parse().ok()?);
                        },
                        "T1" => {
                            t_1 = Some(parts.next()?.parse().ok()?);
                        },
                        "STATUS" => {
                            let raw_status: u32 = parts.next()?.parse().ok()?;
                            status = Status::from_bits(raw_status);
                        },
                        _ => None?
                    }
                }
                Some((SerialCommandResult::G1{ tds_1, ph_1, t_1, status }, success))
            },
            cmd => {
                warn!("Unknown command: {:?}", cmd);
                None
            }
        }
    }
}

pub enum SerialCommand {
    /// Get raw sensore values
    G0,
    /// Get filtred sensore values
    G1,
    S0 {
        on: bool,
    },
}

impl Display for SerialCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SerialCommand::G0 => write!(f, "G0"),
            SerialCommand::G1 => write!(f, "G1"),
            SerialCommand::S0 { on} => write!(f, "S0 {}", if *on {"ON"} else {"OFF"}),
        }
    }
}

impl SerialDaemonHandle {
    pub fn send(&mut self, cmd: SerialCommand) -> std::io::Result<()> {
        self.port.write_fmt(format_args!("{}\n", cmd))
    }
}