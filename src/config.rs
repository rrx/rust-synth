use serde_derive::Deserialize;
use std::fs;
use std::collections::{HashMap, HashSet};
use notify::{Watcher, DebouncedEvent, RecursiveMode, watcher};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::sync::Arc;
use super::config;
use super::midi;
use super::message;

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub enum DeviceType {
    #[serde(rename="hardware")]
    Hardware,
    #[serde(rename="virtual")]
    Virtual
}
impl Default for DeviceType {
    fn default() -> Self {
        Self::Hardware
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Device {
    pub key: String,
    pub name: String,
    #[serde(default="empty_string")]
    pub description: String,
    #[serde(rename="type", default="DeviceType::default")]
    pub device_type: DeviceType,
    #[serde(default="default_false")]
    pub input: bool,
    #[serde(default="default_false")]
    pub output: bool,
    pub mapping: Option<Vec<DeviceMap>>
}
impl Device {
    pub fn mappings(&self) -> Vec<ParsedDeviceMap> {
        self.mapping.as_ref().unwrap_or(&vec![]).iter()
        .map(|m| m.parse())
        .filter(|m| m.is_some())
        .map(|m| m.unwrap())
        .collect()
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Sound {
    pub key: String,
    #[serde(default="empty_string")]
    pub description: String,
    #[serde(default="default_seq")]
    pub seq: u8,
    pub path: String
}

fn empty_string() -> String {
    "".to_string()
}

fn default_seq() -> u8 {
    0
}

fn default_false() -> bool {
    false
}

#[derive(Deserialize, Debug, Clone)]
pub struct DeviceMap {
    sound: Option<String>,
    note: Option<u8>,
    channel: Option<u8>,
    controller: Option<u8>,
    #[serde(default="default_false")]
    aftertouch: bool,
    signal: Option<String>,
    min: Option<f32>,
    max: Option<f32>,
    forward: Option<String>,
    offset: Option<i8>
}

#[derive(Debug, Clone)]
pub enum ParsedDeviceMap {
    SoundMap { key: String, note: u8, channel: Option<u8> },
    Aftertouch { signal: String, min: Option<f32>, max: Option<f32> },
    Controller { controller: u8, signal: String, min: Option<f32>, max: Option<f32> },
    Forward { channel: Option<u8>, forward: String, offset: Option<i8> }
}

impl DeviceMap {
    fn parse(&self) -> Option<ParsedDeviceMap> {
        if self.aftertouch {
            let signal = self.signal.as_ref().unwrap();
            return Some(ParsedDeviceMap::Aftertouch { signal: signal.clone(), min: self.min, max: self.max })
        }

        if let Some(forward) = &self.forward {
            return Some(ParsedDeviceMap::Forward { forward: forward.clone(), channel: self.channel, offset: self.offset })
        }

        if let Some(key) = &self.sound {
            return Some(ParsedDeviceMap::SoundMap { key: key.clone(), note: self.note.unwrap(), channel: self.channel })
        }

        if let Some(controller) = self.controller {
            return Some(ParsedDeviceMap::Controller { controller, signal: self.signal.as_ref().unwrap().clone(), min: self.min, max: self.max })
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct Sounds {
    h: HashMap<String,Vec<Sound>>
}

impl Default for Sounds {
    fn default() -> Self {
        Sounds { h: HashMap::default() }
    }
}

impl Sounds {
    pub fn load(filename: &str) -> Sounds {
        let mut sounds = Sounds::default();
        let s = fs::read_to_string(filename).unwrap_or("".to_string());
        let cfg: ConfigLoader = toml::from_str(&s).unwrap_or(ConfigLoader::default());
        cfg.sound.as_ref().unwrap_or(&vec![]).iter().for_each(|s| {
            if !sounds.h.contains_key(&s.key) {
                sounds.h.insert(s.key.clone(), vec![]);
            }
            if let Some(sound) = sounds.h.get_mut(&s.key) {
                sound.push(s.clone());
            }
        });
        sounds
    }
    pub fn get(&self, key: &String, seq: u8) -> Option<Sound> {
        let v = self.h.get(key);
        match v.unwrap_or(&vec![]).get(seq as usize) {
            Some(sound) => Some(sound.clone()),
            None => None
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigLoader {
    pub device: Option<Vec<Device>>,
    pub sound: Option<Vec<Sound>>
}
impl Default for ConfigLoader {
    fn default() -> Self {
        Self { device: None, sound: None }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub devices: Vec<Device>,
    pub sounds: Sounds
}

impl Config {
    pub fn load() -> Self {
        let s = fs::read_to_string("run.toml").unwrap_or("".to_string());
        let data: ConfigLoader = toml::from_str(&s).unwrap_or(ConfigLoader::default());
        let sounds = Sounds::load("sounds.toml");
        Self { devices: data.device.unwrap_or(vec![]), sounds }
    }

    pub fn hardware_inputs(&self) -> Vec<Device> {
        self.devices.iter().filter(|v| v.input && v.device_type == DeviceType::Hardware).map(|v| v.clone()).collect::<Vec<Device>>()
    }
    pub fn virtual_inputs(&self) -> Vec<Device> {
        self.devices.iter().filter(|v| v.input && v.device_type == DeviceType::Virtual).map(|v| v.clone()).collect::<Vec<Device>>()
    }
    pub fn hardware_outputs(&self) -> Vec<Device> {
        self.devices.iter().filter(|v| v.output && v.device_type == DeviceType::Hardware).map(|v| v.clone()).collect::<Vec<Device>>()
    }
    pub fn virtual_outputs(&self) -> Vec<Device> {
        self.devices.iter().filter(|v| v.output && v.device_type == DeviceType::Virtual).map(|v| v.clone()).collect::<Vec<Device>>()
    }
    pub fn mappings(&self) -> Vec<ParsedDeviceMap> {
        self.devices.iter()
        .filter(|d| d.mapping.is_some())
        .map(|d| d.mapping.as_ref().unwrap())
        .flatten()
        .map(|m| m.parse())
        .filter(|m| m.is_some())
        .map(|m| m.unwrap())
        .collect()
    }
}

pub fn config_watch_thread(events: message::Events) {
    // Create a channel to receive the events.
    let (watch_tx, watch_rx) = channel();

    // Create a watcher object, delivering debounced events.
    // The notification back-end is selected based on the platform.
    let mut watcher = watcher(watch_tx, Duration::from_secs(2)).unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch("run.toml", RecursiveMode::NonRecursive).unwrap();
    watcher.watch("sounds.toml", RecursiveMode::NonRecursive).unwrap();

    loop {
        match watch_rx.recv() {
            Ok(DebouncedEvent::NoticeWrite(event)) => {
                println!("{:?}", event);
                let cfg = Arc::new(config::Config::load());
                events.midi_tx.send(midi::AppMidiEvent::ConfigUpdate(cfg.clone())).unwrap();
                events.app_tx.send(message::Message::ConfigUpdate(cfg)).unwrap();
            }
            Ok(event) => {
                println!("{:?}", event);
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }
}


