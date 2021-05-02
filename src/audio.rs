use crate::vsthost::VSTHost;
use vst::host::HostBuffer;
use std::collections::HashMap;
use crossbeam::channel::{Sender, Receiver, unbounded};
use super::config::*;
use std::sync::Arc;

pub struct AudioData {
    pub(crate) sounds: Vec<audrey::read::BufFileReader>,
    signals: HashMap<String, f32>,
    pub(crate) audio_tx: Sender<AudioMessage>,
    pub(crate) audio_rx: Receiver<AudioMessage>,
}

impl Default for AudioData {
    fn default() -> Self {
        let (audio_tx, audio_rx) = unbounded();
        Self {
            sounds: vec![],
            audio_tx,
            audio_rx,
            signals: HashMap::new()
        }
    }
}

pub struct Audio {
    // host: VSTHost,
    host_buffer: HostBuffer<f32>,
    // host: cpal::Host,
    // output: cpal::Device,
    stream: nannou_audio::Stream<AudioData>,
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
            // sounds: vec![],
            // host,
            host_buffer,
            audio_rx,
            audio_tx,
            // signals: HashMap::new(),
            stream: audio_stream
            // host,
            // output
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
pub fn audio(data: &mut AudioData, buffer: &mut nannou_audio::Buffer) {
    let mut have_ended = vec![];
    let len_frames = buffer.len_frames();

    // process messages
    match data.audio_rx.try_recv() {
        Ok(AudioMessage::SoundOn { id, sound }) => {
            data.sounds.push(sound);
        }
        Ok(AudioMessage::SignalUpdate { key, value }) => {
            data.signals.insert(key, value);
        }
        _ => ()
    }

    let volume = data.signals.get("volume").unwrap_or(&0.);

    // Sum all of the sounds onto the buffer.
    for (i, sound) in data.sounds.iter_mut().enumerate() {
        let mut frame_count = 0;
        let file_frames = sound.frames::<[f32; 2]>().filter_map(Result::ok);
        for (frame, file_frame) in buffer.frames_mut().zip(file_frames) {
            for (sample, file_sample) in frame.iter_mut().zip(&file_frame) {
                *sample += *file_sample * volume;
            }
            frame_count += 1;
        }

        // If the sound yielded less samples than are in the buffer, it must have ended.
        if frame_count < len_frames {
            have_ended.push(i);
        }
    }

    // Remove all sounds that have ended.
    for i in have_ended.into_iter().rev() {
        data.sounds.remove(i);
    }
}

pub fn launch_sound(cfg: &Arc<Config>, audio_tx: Sender<AudioMessage>, name: &str) {
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
}


