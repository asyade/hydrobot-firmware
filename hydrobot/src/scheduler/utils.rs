
use actix::prelude::*;
use std::time::{SystemTime, Duration};
use std::collections::VecDeque;

pub struct PulseMonitor {
    pub threshold: f64,
    pub pulse_duration: Duration,
    pub last_pulse: SystemTime,
    pub pulse_minimum_interval: Duration,
    pub suspend: bool,
}


impl PulseMonitor {
    pub fn new(threshold: f64, pulse_minimum_interval: Duration, pulse_duration: Duration) -> Self {
        Self {
            suspend: false,
            threshold,
            pulse_duration,
            last_pulse: std::time::UNIX_EPOCH,
            pulse_minimum_interval,
        }
    }

    pub fn resume(&mut self) {
        self.suspend = false;
    }

    pub fn update(&mut self, current: f64) -> Option<Duration> {
        if self.suspend {
            None
        } else if current > self.threshold && self.last_pulse.elapsed().unwrap_or_default() > self.pulse_minimum_interval {
            self.last_pulse = SystemTime::now();
            self.suspend = true;
            Some(self.pulse_duration)
        } else {
            None
        }
    }
}
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AnalyticStatus {
    Undefined,
    Unknown,
    Stabilizing(f64, SystemTime),
    Stable(f64),
    Uprising(f64),
    Downrising(f64),
}


pub struct SamplesAnalytic {
    pub stabilization_delay: Duration,
    pub samples: VecDeque<(u64, f64)>,
    pub status: AnalyticStatus,
    /// Number of consecutive sample on which calculation is done
    pub history_size: usize,
    /// Minimum delta betwen two sample to assume theme different
    pub presision: i64,
}

impl SamplesAnalytic {
    pub fn new(history_size: usize, presision: i64, stabilization_delay: Duration) -> Self {
        Self {
            stabilization_delay,
            presision,
            history_size,
            samples: VecDeque::with_capacity(history_size),
            status: AnalyticStatus::Undefined,
        }
    }

    pub fn clear(&mut self) {
        self.samples.clear();
        self.status = AnalyticStatus::Unknown;
    }

    fn update_status(&mut self, new_status: AnalyticStatus) -> Option<AnalyticStatus> {
        if new_status != self.status {
            self.status = new_status;
            Some(new_status)
        } else { None }
    }

    pub fn sample(&mut self, instant: u64, sample: f64) -> Option<AnalyticStatus> {
        self.samples.push_front((instant, sample));
        if self.samples.len() > self.history_size {
            self.samples.pop_back();
        }
        if self.samples.len() < self.history_size {
            return self.update_status(AnalyticStatus::Unknown)
        }
        let min = self.samples.iter().map(|(_, e)| e.round() as i64).min().unwrap();
        let max = self.samples.iter().map(|(_, e)| e.round() as i64).max().unwrap();
        let current = if let AnalyticStatus::Stabilizing(e, _) |  AnalyticStatus::Stable(e) | AnalyticStatus::Uprising(e) | AnalyticStatus::Downrising(e) = self.status {
            e.round() as i64
        } else {
            (min as f64 + max as f64 / 2.0).round() as i64
        };
        let uprising_delta = max - current;
        let downrising_delta = current - min;
        let dir = if uprising_delta - downrising_delta > self.presision { 1 } else if downrising_delta - uprising_delta > self.presision { -1 } else { 0 }; 
        let new_status = match self.status {
            AnalyticStatus::Stabilizing(val, from) if dir == 0 && SystemTime::now().duration_since(from).unwrap() > self.stabilization_delay => AnalyticStatus::Stable(val),
            AnalyticStatus::Stabilizing(val, from) if dir == 0 => AnalyticStatus::Stabilizing(val, from),
            AnalyticStatus::Stable(val) if dir == 0 => AnalyticStatus::Stable(val),
            _ if dir == 1 => AnalyticStatus::Uprising((max + current) as f64 / 2.0),
            _ if dir == -1 => AnalyticStatus::Downrising((min + current) as f64 / 2.0),
            _ if dir == 0 => AnalyticStatus::Stabilizing(current as f64, SystemTime::now()),
            e => e,
        };
        self.update_status(new_status)
    }
}