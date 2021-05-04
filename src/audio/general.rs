use crate::audio::parameters::Parameters;

pub struct General {
    params: Parameters<f32>,
}
impl Default for General {
    fn default() -> Self {
        Self { params: Parameters::default() }
    }
}
impl General {
    pub fn param(&mut self, key: &str, value: f32) {
        self.params.update(key, &value);
    }

    pub fn process(&mut self, buffer: &mut nannou_audio::Buffer) {
        let amp = self.params.get("A");
        for frame in buffer.frames_mut() {
            for sample in frame.iter_mut() {
                *sample *= amp;
            }
        }
    }
}


