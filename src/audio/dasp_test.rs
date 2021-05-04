use dasp::slice::ToFrameSliceMut;
use dsp::{Frame, FromSample, Graph, Node, Sample, Walker};
use dsp::daggy::NodeIndex;
use crate::audio::parameters::Parameters;

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

pub struct DaspTestData {
    params: Parameters<f32>,
    graph: Graph<[f32; 2], DspNode>,
    synth: NodeIndex<usize>,
    buf: Vec<FrameType>
}
impl Default for DaspTestData {
    fn default() -> Self {
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
            params: Parameters::default(),
            graph,
            synth,
            buf: Vec::with_capacity(2048)
        }
    }
}
impl DaspTestData {
    pub fn param(&mut self, key: &str, value: f32) {
        self.params.update(key, &value);
    }

    pub fn process(&mut self, buffer: &mut nannou_audio::Buffer) {
        let channels = buffer.channels();
        let sample_rate = buffer.sample_rate();
        let amp = self.params.get("A");
        let rate = self.params.get("R");

        // make sure we have a buffer big enough
        if self.buf.len() < buffer.len_frames() {
            self.buf.resize(buffer.len_frames(), FrameType::EQUILIBRIUM);
        }

        let mut samples = self.buf.as_mut_slice();//into_boxed_slice();
        self.graph.audio_requested(&mut samples, sample_rate as f64);
        for (frame, graph_frame) in buffer.frames_mut().zip(samples.iter()) {
            for (sample, graph_sample) in frame.iter_mut().zip(graph_frame) {
                *sample += graph_sample * amp;
            }
        }
        self.post(buffer, rate);
    }

    fn post(&mut self, buffer: &mut nannou_audio::Buffer, pitch_control: f32) {
        // Traverse inputs or outputs of a node with the following pattern.
        let mut inputs = self.graph.inputs(self.synth);
        while let Some(input_idx) = inputs.next_node(&self.graph) {
            if let DspNode::Oscillator(_, ref mut pitch, _) = self.graph[input_idx] {
                // Pitch down our oscillators for fun.
                let delta: f64 = 0.1 * pitch_control as f64;
                *pitch -= delta;
            }
        }
    }
}
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

