use crate::vsthost::VSTHost;
use vst::host::HostBuffer;
use std::collections::HashMap;
use crossbeam::channel::{Sender, Receiver, unbounded};
use super::config::*;
use std::sync::{Arc, Mutex};

mod sounds;
mod dasp_test;
mod general;
pub(crate) mod parameters;

use parameters::Parameters;

pub struct AudioData<R> {
    pub(crate) sounds: sounds::Sounds,
    params: Parameters<R>,
    pub(crate) audio_tx: Sender<AudioMessage>,
    pub(crate) audio_rx: Receiver<AudioMessage>,
    dasp_test: dasp_test::DaspTestData,
    general: general::General
}

impl<R> Default for AudioData<R>
    where R: dasp::sample::Sample<Float = R> + std::ops::Div
    {
    fn default() -> Self {
        let (audio_tx, audio_rx) = unbounded();
        let mut params = Parameters::default();
        params.update("volume", &R::IDENTITY);
        params.update("pitch", &R::EQUILIBRIUM);
        Self {
            sounds: sounds::Sounds::default(),
            audio_tx,
            audio_rx,
            params,
            dasp_test: dasp_test::DaspTestData::default(),
            general: general::General::default()
        }
    }
}

pub struct Audio {
    // host: VSTHost,
    host_buffer: HostBuffer<f32>,
    // host: cpal::Host,
    // output: cpal::Device,
    stream: nannou_audio::Stream<AudioData<f32>>,
    pub(crate) audio_tx: Sender<AudioMessage>,
    pub(crate) audio_rx: Receiver<AudioMessage>,
}
unsafe impl Send for Audio {}

impl Default for Audio {
    fn default() -> Self {
        // let (audio_tx, audio_rx) = unbounded();
        // let mut host = VSTHost::load("Upright Piano.vst");
        let host_buffer: HostBuffer<f32> = HostBuffer::new(2, 2);
        // let host = cpal::default_host();
        // let output = host.default_output_device().expect("no output device available");
        let audio_host = nannou_audio::Host::new();
        let data = AudioData::default();
        let audio_tx = data.audio_tx.clone();
        let audio_rx = data.audio_rx.clone();
        let audio_stream = audio_host
            .new_output_stream(data)
            .render(crate::audio::audio)
            .build()
            .unwrap();

        Self { 
            host_buffer,
            audio_rx,
            audio_tx,
            stream: audio_stream
        }
    }
}
pub enum AudioMessage {
    SoundOn { id: u64, sound: audrey::read::BufFileReader },
    SoundOff { id: u64 },
    SignalUpdate { key: String, value: f32 }
}

// A function that renders the given `Audio` to the given `Buffer`.
// In this case we play the audio file.
pub fn audio(data: &mut AudioData<f32>, buffer: &mut nannou_audio::Buffer) {
    // process messages
    let messages = pull_messages(data);
    process_messages(data, messages);
    // dsp::slice::equilibrium(buffer);
    data.sounds.param("A", 1.0);
    data.sounds.process(buffer);

    data.dasp_test.param("A", 1.0);
    data.dasp_test.param("R", data.params.get("pitch"));
    data.dasp_test.process(buffer);

    data.general.param("A", data.params.get("volume"));
    data.general.process(buffer);
}

fn pull_messages(data: &AudioData<f32>) -> Vec<AudioMessage> {
    data.audio_rx.try_iter().collect()
}

fn process_messages(data: &mut AudioData<f32>, messages: Vec<AudioMessage>) {
    for m in messages {
        match m {
            AudioMessage::SoundOn { id, sound } => {
                data.sounds.on(id, sound);
            }
            AudioMessage::SoundOff { id } => {
                data.sounds.off(id);
            }
            AudioMessage::SignalUpdate { key, value } => {
                data.params.update(&key, &value);
            }
            _ => ()
        }
    };
}

pub fn launch_sound(cfg: &Arc<Config>, audio_tx: Sender<AudioMessage>, name: &str, on: bool) {
    if on {
        let maybe_sound = cfg.sounds.get(&name.to_string(), 0);
        if let Some(sound) = maybe_sound {
            println!("Load {}, {}", name, sound.path);
            let r_sound = audrey::open(&sound.path);
            if let Ok(s) = r_sound {
                println!("Play {}", name);
                audio_tx.send(AudioMessage::SoundOn {id: 0, sound: s}).unwrap();
            } else {
                println!("Unable to load sound {}", &sound.path);
            }
        } else {
            println!("Sounds not found {}", name);
        }
    } else {
        println!("Stop {}", name);
        audio_tx.send(AudioMessage::SoundOff {id: 0}).unwrap();
    }
}


