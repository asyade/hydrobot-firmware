#[macro_use] extern crate log;
#[macro_use] extern crate failure;
#[macro_use] extern crate bitflags;
use actix::prelude::*;
use serialport::{UsbPortInfo, SerialPortType};
use std::time::{Duration};

pub mod store;
pub mod gui;
pub mod daemon;
pub mod scheduler;
use daemon::*;
use gui::*;
use store::*;
use scheduler::*;
use clap::Clap;

/// This doc string acts as a help message when the user runs '--help'
/// as do all doc strings on fields
#[derive(Clap)]
#[clap(version = "0.1", author = "Asya C. <corbeau.asya.dev@gmail.com>")]
struct Opts {
    #[clap(short, long)]
    daemon: bool,
}

#[actix_rt::main]
async fn main() {
    let opts: Opts = Opts::parse();
    let store = Store::open(std::path::PathBuf::from("./store"));
    let ports = serialport::available_ports().expect("Failed to get serial port list");
    let arduino = ports.into_iter().find(|port| {
        if let SerialPortType::UsbPort(UsbPortInfo { vid: 6790, pid: 29987, ..}) = port.port_type {
            true
        } else {
            false
        }
    });
    if opts.daemon {
        pretty_env_logger::init();
    }
    if let Some(port) = arduino {
        let mut port = serialport::open(&port.port_name).expect("Failed to open serial port !");
        port.set_timeout(Duration::from_secs(10)).expect("Failed to set timeout");
        let scheduler = SchedulerActor::new(store.clone()).start();
        let gui = if opts.daemon { None } else { Some(GuiActor::new(scheduler.clone(), store.clone()).start()) };
        let daemon_handle = SerialDaemon::new(port, scheduler.clone());
        scheduler.do_send(SchedulerRequest::Init { gui, handle: daemon_handle });
        tokio::signal::ctrl_c().await.unwrap();
        info!("Ctrl-C received, shutting down");
        System::current().stop();
    } else {
        error!("No board connected !");
    }
}
