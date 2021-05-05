use crate::audio::parameters::Parameters;
use crate::audio::AudioMessage;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

pub struct SoundEntry {
    id: u64,
    buf: audrey::read::BufFileReader,
    consumed: bool
}
impl SoundEntry {
    fn process(&mut self, buffer: &mut nannou_audio::Buffer) {
        let len_frames = buffer.len_frames();
        let mut frame_count = 0;
        let file_frames = self.buf.frames::<[f32; 2]>().filter_map(Result::ok);
        for (frame, file_frame) in buffer.frames_mut().zip(file_frames) {
            for (sample, file_sample) in frame.iter_mut().zip(&file_frame) {
                *sample += *file_sample;// * sound_amp;
            }
            frame_count += 1;
        }
        // If the sound yielded less samples than are in the buffer, it must have ended.
        if frame_count < len_frames {
            self.consumed = true;
        }
    }
}

pub struct Sounds {
    params: Parameters<f32>,
    pub(crate) sounds: HashMap<u64, SoundEntry>
}
impl Default for Sounds {
    fn default() -> Self {
        Self { sounds: HashMap::new(), params: Parameters::default() }
    }
}
impl Sounds {
    pub fn param(&mut self, key: &str, value: f32) {
        self.params.update(key, &value);
    }

    pub fn on(&mut self, id: u64, buf: audrey::read::BufFileReader) {
        self.sounds.insert(id, SoundEntry { id, buf, consumed: false });
    }

    pub fn off(&mut self, id: u64) {
        self.sounds.remove(&id);
    }

    fn consume(&mut self, buffer: &mut nannou_audio::Buffer) -> Vec<u64> {
        // Sum all of the sounds onto the buffer.
        self.sounds.values_mut().filter(|sound| !sound.consumed).filter_map(|sound| {
            sound.process(buffer);
            if sound.consumed {
                Some(sound.id)
            } else {
                None
            }
        }).collect::<Vec<_>>()
    }

    pub fn process(&mut self, buffer: &mut nannou_audio::Buffer) {
        for id in self.consume(buffer) {
            self.sounds.remove(&id);
        }
    }
}

