use std::path::Path;
use sled::*;
use std::time::{SystemTime, Duration};

const SETTING_TDS_1: &str = "tds_1_thresh";
const SETTING_TDS_1_DEFAULT: f64 = 500.0;

const SETTING_OSMOSEUR_PULSE_DURATION: &str = "osmoseur_pulse_duration";
const SETTING_OSMOSEUR_PULSE_DURATION_DEFAULT: u64 = 10; //10 secs

const SETTING_OSMOSEUR_PULSE_MIN_INTERVAL: &str = "osmoseur_pulse_min_interval";
const SETTING_OSMOSEUR_PULSE_MIN_INTERVAL_DEFAULT: u64 = 240; //10 secs

const SETTING_PH_1: &str = "ph_1_thresh";
const SETTING_PH_1_DEFAULT: f64 = 7.0;

const SETTING_PH_PULSE_DURATION: &str = "ph_pulse_duration";
const SETTING_PH_PULSE_DURATION_DEFAULT: u64 = 10; //10 secs

const SETTING_PH_PULSE_MIN_INTERVAL: &str = "ph_pulse_min_interval";
const SETTING_PH_PULSE_MIN_INTERVAL_DEFAULT: u64 = 240; //10 secs

const SETTING_TDS_MONITORING: &str = "tds_monitoring";
const SETTING_PH_MONITORING: &str = "ph_monitoring";

#[derive(Clone)]
pub struct Store {
    pub tds_1_tree: sled::Tree,
    pub settings_tree: sled::Tree,
    db: sled::Db,
}

impl Store {
    pub fn open<T: AsRef<Path>>(path: T) -> Store {
        let db = sled::open(path).expect("Can't open store !");
        Self {
            settings_tree: db.open_tree("settings").expect("Failed to open settings tree !"),
            tds_1_tree: db.open_tree("tds_1").expect("Failed to open tds tree !"),
            db,
        }
    }

    fn put_setting_bool(&self, name: &str, val: bool) {
        self.settings_tree.insert(name, &[val as u8]).expect("Failed to update param");
        let _ = self.db.flush();
    }

    fn get_setting_bool(&self, name: &str, default: bool) -> bool {
        if let Ok(Some(param)) = self.settings_tree.get(name) {
            let buff: &[u8] = param.as_ref();
            buff[0] == 1
        } else {
            self.put_setting_bool(name, default);
            default
        }
    }

    fn put_setting_f64(&self, name: &str, val: f64) {
        self.settings_tree.insert(name, &val.to_be_bytes()).expect("Failed to update param");
        let _ = self.db.flush();
    }

    fn get_setting_f64(&self, name: &str, default: f64) -> f64 {
        if let Ok(Some(param)) = self.settings_tree.get(name) {
            let buff: &[u8] = param.as_ref();
            f64::from_be_bytes([buff[0], buff[1], buff[2], buff[3], buff[4], buff[5], buff[6], buff[7] ])
        } else {
            self.put_setting_f64(name, default);
            default
        }
    }

    fn put_setting_i64(&self, name: &str, val: i64) {
        self.settings_tree.insert(name, &val.to_be_bytes()).expect("Failed to update param");
        let _ = self.db.flush();
    }

    fn get_setting_i64(&self, name: &str, default: i64) -> i64 {
        if let Ok(Some(param)) = self.settings_tree.get(name) {
            let buff: &[u8] = param.as_ref();
            i64::from_be_bytes([buff[0], buff[1], buff[2], buff[3], buff[4], buff[5], buff[6], buff[7] ])
        } else {
            self.put_setting_i64(name, default);
            default
        }
    }

    fn put_setting_u64(&self, name: &str, val: u64) {
        self.settings_tree.insert(name, &val.to_be_bytes()).expect("Failed to update param");
        let _ = self.db.flush();
    }

    fn get_setting_u64(&self, name: &str, default: u64) -> u64 {
        if let Ok(Some(param)) = self.settings_tree.get(name) {
            let buff: &[u8] = param.as_ref();
            u64::from_be_bytes([buff[0], buff[1], buff[2], buff[3], buff[4], buff[5], buff[6], buff[7] ])
        } else {
            self.put_setting_u64(name, default);
            default
        }
    }
    
    pub fn set_tds_monitoring(&self, val: bool) {
        self.put_setting_bool(SETTING_TDS_MONITORING, val)
    }
    pub fn get_tds_monitoring(&self) -> bool {
        self.get_setting_bool(SETTING_TDS_MONITORING, false)
    }

    pub fn set_ph_monitoring(&self, val: bool) {
        self.put_setting_bool(SETTING_PH_MONITORING, val)
    }
    pub fn get_ph_monitoring(&self) -> bool {
        self.get_setting_bool(SETTING_PH_MONITORING, false)
    }

    pub fn set_tds_1_thresh(&self, val: f64) {
        self.put_setting_f64(SETTING_TDS_1, val)
    }
    pub fn get_tds_1_thresh(&self) -> f64 {
        self.get_setting_f64(SETTING_TDS_1, SETTING_TDS_1_DEFAULT)
    }
    pub fn set_osmoseur_pulse_duration(&self, val: Duration ) {
        self.put_setting_u64(SETTING_OSMOSEUR_PULSE_DURATION, val.as_secs())
    }
    pub fn get_osmoseur_pulse_duration(&self) -> Duration {
        Duration::from_secs(self.get_setting_u64(SETTING_OSMOSEUR_PULSE_DURATION, SETTING_OSMOSEUR_PULSE_DURATION_DEFAULT))
    }
    pub fn set_osmoseur_pulse_min_interval(&self, val: Duration ) {
        self.put_setting_u64(SETTING_OSMOSEUR_PULSE_MIN_INTERVAL, val.as_secs())
    }
    pub fn get_osmoseur_pulse_min_interval(&self) -> Duration {
        Duration::from_secs(self.get_setting_u64(SETTING_OSMOSEUR_PULSE_MIN_INTERVAL, SETTING_OSMOSEUR_PULSE_MIN_INTERVAL_DEFAULT))
    }

    pub fn set_ph_1_thresh(&self, val: f64) {
        self.put_setting_f64(SETTING_PH_1, val)
    }
    pub fn get_ph_1_thresh(&self) -> f64 {
        self.get_setting_f64(SETTING_PH_1, SETTING_PH_1_DEFAULT)
    }
    pub fn set_ph_pulse_duration(&self, val: Duration ) {
        self.put_setting_u64(SETTING_PH_PULSE_DURATION, val.as_secs())
    }
    pub fn get_ph_pulse_duration(&self) -> Duration {
        Duration::from_secs(self.get_setting_u64(SETTING_PH_PULSE_DURATION, SETTING_PH_PULSE_DURATION_DEFAULT))
    }
    pub fn set_ph_pulse_min_interval(&self, val: Duration ) {
        self.put_setting_u64(SETTING_PH_PULSE_MIN_INTERVAL, val.as_secs())
    }
    pub fn get_ph_pulse_min_interval(&self) -> Duration {
        Duration::from_secs(self.get_setting_u64(SETTING_PH_PULSE_MIN_INTERVAL, SETTING_PH_PULSE_MIN_INTERVAL_DEFAULT))
    }

    pub fn insert_tds_1_metric(&self, when: SystemTime, sample: f64) {
        let timestamp = when.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let _ = self.tds_1_tree.insert(&timestamp.to_le_bytes(), &sample.to_le_bytes());
    }

    pub fn get_fresh_tds_1_metric(&self, buffer: &mut Vec<f64>, mut limit: usize) {
        let last_key = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        while let Ok(Some((key, val))) = self.tds_1_tree.get_lt(&last_key.to_le_bytes()) {
            let val: &[u8] = val.as_ref();
            buffer.push(f64::from_le_bytes([val[0], val[1], val[2], val[3], val[4], val[5], val[6], val[7]]));
            if limit <= 1 { break; } else { limit -= 1 };
        }
    }
}