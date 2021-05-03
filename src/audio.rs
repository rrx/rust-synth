use crate::vsthost::VSTHost;
use vst::host::HostBuffer;
use std::collections::HashMap;
use crossbeam::channel::{Sender, Receiver, unbounded};
use super::config::*;
use std::sync::{Arc, Mutex};
use dasp::slice::ToFrameSliceMut;
use dsp::{Frame, FromSample, Graph, Node, Sample, Walker};
use dsp::daggy::NodeIndex;

pub struct AudioData<R> {
    pub(crate) sounds: Vec<audrey::read::BufFileReader>,
    signals: HashMap<String, R>,
    pub(crate) audio_tx: Sender<AudioMessage>,
    pub(crate) audio_rx: Receiver<AudioMessage>,
    graph: Graph<[f32; 2], DspNode>,
    synth: NodeIndex<usize>
    // streams: Vec<StreamData<R>>,
}
const CHANNELS: usize = 2;
const A5_HZ: Frequency = 440.0;
const D5_HZ: Frequency = 587.33;
const F5_HZ: Frequency = 698.46;
/// SoundStream is currently generic over i8, i32 and f32. Feel free to change it!
type Output = f32;
type FrameType = [Output; CHANNELS];
type Phase = f64;
type Frequency = f64;
type Volume = f32;

/// Return a sine wave for the given phase.
fn sine_wave<S: Sample>(phase: Phase, volume: Volume) -> S
where
    S: Sample + FromSample<f32>,
{
    use std::f64::consts::PI;
    ((phase * PI * 2.0).sin() as f32 * volume).to_sample::<S>()
}
/// Our type for which we will implement the `Dsp` trait.
#[derive(Debug)]
enum DspNode {
    /// Synth will be our demonstration of a master GraphNode.
    Synth,
    /// Oscillator will be our generator type of node, meaning that we will override
    /// the way it provides audio via its `audio_requested` method.
    Oscillator(Phase, Frequency, Volume),
}
impl Node<FrameType> for DspNode {
    /// Here we'll override the audio_requested method and generate a sine wave.
    fn audio_requested(&mut self, buffer: &mut [FrameType], sample_hz: f64) {
        match *self {
            DspNode::Synth => (),
            DspNode::Oscillator(ref mut phase, frequency, volume) => {
                dsp::slice::map_in_place(buffer, |_| {
                    let val = sine_wave(*phase, volume);
                    *phase += frequency / sample_hz;
                    Frame::from_fn(|_| val)
                });
            }
        }
    }
}
impl<R> Default for AudioData<R> {
    fn default() -> Self {
        let (audio_tx, audio_rx) = unbounded();
        let mut graph = Graph::new();
        let synth = graph.add_node(DspNode::Synth);
        // Connect a few oscillators to the synth.
        let (_, oscillator_a) = graph.add_input(DspNode::Oscillator(0.0, A5_HZ, 0.2), synth);
        graph.add_input(DspNode::Oscillator(0.0, D5_HZ, 0.1), synth);
        graph.add_input(DspNode::Oscillator(0.0, F5_HZ, 0.15), synth);
        // If adding a connection between two nodes would create a cycle, Graph will return an Err.
        if let Err(err) = graph.add_connection(synth, oscillator_a) {
            println!("Testing for cycle error: {}", &err);
        }
        // Set the synth as the master node for the graph.
        graph.set_master(Some(synth));

        Self {
            sounds: vec![],
            audio_tx,
            audio_rx,
            signals: HashMap::new(),
            // streams: vec![]
            graph,
            synth
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
pub fn audio(data: &mut AudioData<f32>, buffer: &mut nannou_audio::Buffer) {
    let mut have_ended = vec![];
    let len_frames = buffer.len_frames();

    let channels = buffer.channels();
    let sample_rate = buffer.sample_rate();

    dsp::slice::equilibrium(buffer);
    let mut v: Vec<FrameType> = Vec::with_capacity(buffer.len_frames());
    v.resize(buffer.len_frames(), FrameType::EQUILIBRIUM);
    let mut samples = v.into_boxed_slice();
    data.graph.audio_requested(&mut samples, sample_rate as f64);

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

    let volume = data.signals.get("volume").unwrap_or(&0.5);

    for (frame, graph_frame) in buffer.frames_mut().zip(samples.iter()) {
        for (sample, graph_sample) in frame.iter_mut().zip(graph_frame) {
            *sample += graph_sample * volume;
        }
    }

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

    let pitch_control = *data.signals.get("pitch").unwrap_or(&0.);

    // Traverse inputs or outputs of a node with the following pattern.
    let mut inputs = data.graph.inputs(data.synth);
    while let Some(input_idx) = inputs.next_node(&data.graph) {
        if let DspNode::Oscillator(_, ref mut pitch, _) = data.graph[input_idx] {
            // Pitch down our oscillators for fun.
            *pitch -= 0.1 * pitch_control as f64;
        }
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


