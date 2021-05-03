use arrayvec::ArrayVec;
use crossbeam::channel::{Sender, Receiver, unbounded};
use std::sync::Arc;
use midir;
use midir::{MidiInput, MidiOutput, Ignore};
use midir::os::unix::{VirtualInput, VirtualOutput};
use midly::{live::LiveEvent, MidiMessage};
use std::collections::HashMap;
use nannou::math::map_range;

use super::config::*;
use super::audio::*;
use super::message;

pub struct MidiModel {
    // pub(crate) inputs: Vec<NamedInputConnection>,
    // outputs: Vec<NamedOutputConnection>,
    pub(crate) midi_rx: Receiver<AppMidiEvent>,
    pub(crate) midi_tx: Sender<AppMidiEvent>,
}
impl Default for MidiModel {
    fn default() -> Self {
        let (midi_tx, midi_rx) = unbounded();
        // Self { inputs: vec![] }
        Self { midi_rx, midi_tx }
    }
}

// #[derive(Debug)]
pub struct NamedOutputConnection {
    key: String,
    name: String,
    conn: midir::MidiOutputConnection
}
impl NamedOutputConnection {
    pub fn close(self) {
        self.conn.close();
    }
}

// #[derive(Debug)]
pub struct NamedInputConnection {
    key: String,
    name: String,
    conn: midir::MidiInputConnection<MidiInputData>
}
impl NamedInputConnection {
    pub fn close(self) {
        self.conn.close();
    }
}

pub struct MidiInputData {
    pub midi_tx: Sender<AppMidiEvent>,
    pub audio_tx: Sender<AudioMessage>,
    pub device: Device,
    pub mappings: Vec<ParsedDeviceMap>,
    pub sound_mappings: HashMap<String, ParsedDeviceMap>,
    pub cfg: Arc<Config>
}
impl MidiInputData {
    pub fn new(midi_tx: Sender<AppMidiEvent>, audio_tx: Sender<AudioMessage>, cfg: Arc<Config>, device: Device) -> Self {
        let mut sound_mappings = HashMap::new();
        let mappings = device.mappings();
        for m in mappings.clone() {
            match m {
                ParsedDeviceMap::SoundMap { ref key, note, channel } => {
                    if let Some(c) = channel {
                        let key1 = format!("{}_{}", note, c);
                        sound_mappings.insert(key1, m.clone());
                    }

                    let key2 = format!("{}", note);
                    sound_mappings.insert(key2, m.clone());
                }
                _ => ()
            }
        }
        Self { midi_tx, audio_tx, device, mappings, cfg, sound_mappings}
    }

    pub fn send_sound(&self, id: u64, path: String) {
        let r_sound = audrey::open(&path);
        if let Ok(s) = r_sound {
            println!("Send {} {}", id, path);
            self.audio_tx.send(AudioMessage::SoundOn {id, sound: s}).unwrap();
        } else {
            println!("Unable to open sound: {}", path);
        }
    }

    pub fn send_signal(&self, key: String, value: f32) {
        self.audio_tx.send(AudioMessage::SignalUpdate {key, value}).unwrap();
    }

    pub fn handle(&self, ts: u64, message: &[u8]) {
        let event = LiveEvent::parse(message).unwrap();
        println!("[{}] MidiRX({}): {:?}", ts, &self.device.key, event);
        match event {
            LiveEvent::Midi { channel, message: MidiMessage::NoteOff { key: note, vel: _ }} => {
            }
            LiveEvent::Midi { channel, message: MidiMessage::NoteOn { key: note, vel: _ }} => {
                let key1 = format!("{}_{}", note, channel);
                let key2 = format!("{}", note);
                println!("x: {} {}", key1, key2);
                for k in [key1, key2].iter() {
                    if let Some(ParsedDeviceMap::SoundMap { key, note, channel }) = self.sound_mappings.get(&k.to_string()) {
                        println!("map: {} {} {:?}", key, note, channel);
                        if let Some(sound) = self.cfg.sounds.get(&key, 0) {
                            self.send_sound(ts, sound.path);
                        } 
                    }
                }
            }

            LiveEvent::Midi { channel, message: MidiMessage::ChannelAftertouch { vel }} => {
                self.mappings.iter().for_each(|m| {
                    match m {
                        ParsedDeviceMap::Aftertouch { signal, min: maybe_min, max: maybe_max } => {
                            let v = if maybe_min.is_some() && maybe_max.is_some() {
                                let max = maybe_max.unwrap();
                                let min = maybe_min.unwrap();
                                map_range(vel.as_int(), 0, 127, min, max)
                            } else {
                                vel.as_int() as f32
                            };
                            println!("Signal {} - {}", signal, v);
                            self.send_signal(signal.to_string(), v);
                        }
                        _ => ()
                    }
                });
            }

            LiveEvent::Midi { channel, message: MidiMessage::Controller { controller: c1, value }} => {
                self.mappings.iter().for_each(|m| {
                    match m {
                        ParsedDeviceMap::Controller { controller: c2, signal, min: maybe_min, max: maybe_max } if c1.as_int() == *c2 => {
                            let v = if maybe_min.is_some() && maybe_max.is_some() {
                                let max = maybe_max.unwrap();
                                let min = maybe_min.unwrap();
                                map_range(value.as_int(), 0, 127, min, max)
                            } else {
                                value.as_int() as f32
                            };
                            println!("Signal {} - {}", signal, v);
                            self.send_signal(signal.to_string(), v);
                        }
                        _ => ()
                    }
                });
            }
            _ => ()
        };

        if message.len() <= 4 {
            let midi_event = MidiEvent::new(ts, &self.device.key, message);
            self.midi_tx.send(AppMidiEvent::Midi(midi_event)).unwrap();
        }
    }
}


#[derive(Debug,Clone)]
pub struct MidiEvent {
    pub(crate) ts: u64,
    pub(crate) key: String,
    pub(crate) b: ArrayVec<u8, 20>
}
impl MidiEvent {
    pub fn new(ts: u64, key: &String, message: &[u8]) -> Self {
        let mut out = ArrayVec::<u8, 20>::new();
        out.try_extend_from_slice(&message).unwrap();
        Self { ts, key: key.clone(), b: out }
    }
}

#[derive(Debug,Clone)]
pub enum AppMidiEvent {
    ConfigUpdate(Arc<Config>),
    Midi(MidiEvent)
}

pub fn create_output_connections(cfg: Arc<Config>) -> Vec<NamedOutputConnection> {
    let outputs: HashMap<String, Device> = cfg.hardware_outputs().into_iter().map(|d| (d.name.clone(), d)).collect();

    let midi_out = MidiOutput::new("Test").unwrap();
    let midi_ports = midi_out.ports();
    let mut output_connections: Vec<NamedOutputConnection> = vec![];

    for (_, p) in midi_ports.iter().enumerate() {
        let m = MidiOutput::new("Test").unwrap();
        let name = m.port_name(p).unwrap();
        if let Some(device) = outputs.get(&name) {
            let conn = m.connect(&p, "out").unwrap();
            println!("Hardware Output {}", name);
            output_connections.push(NamedOutputConnection { key: device.key.clone(), name: name.to_string(), conn });
        }
    }

    cfg.virtual_outputs().iter().for_each(|device| {
        println!("Virtual Output {}", &device.name);
        let v_midi_out = MidiOutput::new("Test").unwrap();
        let conn = v_midi_out.create_virtual(&device.name).unwrap();
        output_connections.push(NamedOutputConnection { key: device.key.clone(), name: device.name.clone(), conn });
    });
    output_connections
}

pub fn recv_midi_thread(cfg: Arc<Config>, events: message::Events) {
    let mut output_connections = create_output_connections(cfg.clone());
    let mut inputs = scan_inputs(cfg.clone(), events.midi_tx.clone(), events.audio_tx.clone());

    loop {
        match events.midi_rx.try_recv() {
            Ok(AppMidiEvent::ConfigUpdate(new_config)) => {
                for o in output_connections {
                    o.close();
                }
                output_connections = create_output_connections(new_config.clone());
                println!("Midi outputs reset");
                inputs.drain(..).for_each(|i| i.close());
                inputs = scan_inputs(new_config, events.midi_tx.clone(), events.audio_tx.clone());
            }
            Ok(AppMidiEvent::Midi(e)) => {
                for o in &mut output_connections {
                    println!("[{}] MidiTX({}): {:?}", e.ts, o.key, e);
                    o.conn.send(&e.b.as_slice()).unwrap();
                }
            }
            _ => ()
        }
    }
}

pub fn scan_inputs(cfg: Arc<Config>, midi_tx: Sender<AppMidiEvent>, audio_tx: Sender<AudioMessage>) -> Vec<NamedInputConnection> {
    let mut input_connections = vec![];
    let mut midi_in = MidiInput::new("Test").unwrap();
    midi_in.ignore(Ignore::None);
    let midi_in_ports = midi_in.ports();
    println!("Available input ports:");
    
    let inputs: HashMap<String, Device> = cfg.hardware_inputs().into_iter().map(|d| (d.name.clone(), d)).collect();
    for (i, p) in midi_in_ports.iter().enumerate() {
        let name = midi_in.port_name(p).unwrap();
        if let Some(device) = inputs.get(&name) {
            println!("{}: {}", i, name);
            let midi_in = MidiInput::new("Test").unwrap();
            let data = MidiInputData::new(midi_tx.clone(), audio_tx.clone(), cfg.clone(), device.clone());

            let conn = midi_in.connect(&p.clone(), "forward", | ts, message, data| {
                data.handle(ts, message);
            }, data).unwrap();
            input_connections.push(NamedInputConnection { key: device.key.clone(), name: name.to_string(), conn});
        }
    }

    cfg.virtual_inputs().iter().for_each(|device| {
        let v_midi_in = MidiInput::new("Test").unwrap();
        println!("Create Virtual Input {}", &device.name);

        let data = MidiInputData::new(midi_tx.clone(), audio_tx.clone(), cfg.clone(), device.clone());
        let conn = v_midi_in.create_virtual(&device.name, |ts, message, data| {
            let event = LiveEvent::parse(message).unwrap();
            println!("[{}] MidiRX({}): {:?}", ts, &data.device.key, event);
            // if message.len() == 3 {
            //     tx.send(Message::Midi(MidiEvent { ts, a: message[0], b: message[1], c: message[2] })).unwrap();
            // }
        }, data).unwrap();
        input_connections.push(NamedInputConnection { key: device.key.clone(), name: device.name.clone(), conn});
    });

    input_connections
}

