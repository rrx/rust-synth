use std::sync::Arc;
use crossbeam::channel::{Sender, Receiver, unbounded};

use super::config;
use super::midi;
use super::audio;

#[derive(Debug,Clone)]
pub enum Message {
    ConfigUpdate(Arc<config::Config>),
    // Quit
}

#[derive(Debug,Clone)]
pub struct Events {
    pub(crate) app_rx: Receiver<Message>,
    pub(crate) app_tx: Sender<Message>,
    pub(crate) midi_rx: Receiver<midi::AppMidiEvent>,
    pub(crate) midi_tx: Sender<midi::AppMidiEvent>,
    pub(crate) audio_rx: Receiver<audio::AudioMessage>,
    pub(crate) audio_tx: Sender<audio::AudioMessage>,
}


