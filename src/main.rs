use nannou::prelude::*;
use crossbeam::channel::unbounded;
use std::sync::Arc;

mod config;
mod vsthost;
mod audio;
mod midi;
mod message;


struct Model {
    cfg: Arc<config::Config>,
    // stream: nannou_audio::Stream<audio::Audio>,
    threads: Vec<std::thread::JoinHandle<()>>,
    events: message::Events,
    midi: midi::MidiModel,
    audio: audio::Audio
}
impl Default for Model {
    fn default() -> Self {
        let cfg = Arc::new(config::Config::load());
        let (app_tx, app_rx) = unbounded();

        let audio_model = audio::Audio::default();
        let audio_rx = audio_model.audio_rx.clone();
        let audio_tx = audio_model.audio_tx.clone();

        let midi_model = midi::MidiModel::default();
        let midi_rx = midi_model.midi_rx.clone();
        let midi_tx = midi_model.midi_tx.clone();

        Self {
            cfg,
            // inputs: vec![],
            // stream: audio_stream,
            threads: vec![],
            midi: midi_model,
            audio: audio_model,
            events: message::Events {
                app_rx: app_rx.clone(),
                app_tx: app_tx.clone(),
                midi_rx,
                midi_tx,
                audio_rx: audio_rx,
                audio_tx: audio_tx
            }
        }
    }
}

impl Model {
    fn start(&mut self) {
        {
            let cfg = self.cfg.clone();
            let events = self.events.clone();
            self.threads.push(std::thread::spawn(move || midi::recv_midi_thread(cfg, events)));
        }
        {
            let events = self.events.clone();
            self.threads.push(std::thread::spawn(move || config::config_watch_thread(events)));
        }
    }
}

fn main() {
    nannou::app(model)
        .event(event)
        .update(update)
        .simple_window(view)
        .run();
}

fn model(app: &App) -> Model {
    app.set_loop_mode(LoopMode::Wait);
    app.set_fullscreen_on_shortcut(true);

    let mut model = Model::default();
    model.start();
    model
}

fn event(app: &App, model: &mut Model, event: Event) {
    // println!("E: {:?}", event);
    use Key::*;
    match event {
        // Event::WindowEvent { id: _, simple: Some(KeyPressed(Key::Q)) } => {
        //     app.quit();
        // }
        Event::WindowEvent { id: _, simple: Some(KeyPressed(key)) } if key >= Key1 && key <= Key0 => {
            let n1 = key as u32;
            let n0 = Key1 as u32;
            let n = (n1 - n0 + 1) % 10;
            let s = format!("{}", n);
            println!("Key#: {:?}/{}", key, &s);
            audio::launch_sound(&model.cfg, model.events.audio_tx.clone(), &s);
        }
        Event::WindowEvent { id: _, simple: Some(KeyPressed(key)) } => {
            let k = format!("{:?}", key);
            let v = key as u32;
            println!("Key: {}/{}", k, v);
            audio::launch_sound(&model.cfg, model.events.audio_tx.clone(), &k);
        }

        // KeyReleased(_key) => {}
        // MouseMoved(_pos) => {}
        // MousePressed(_button) => {}
        // MouseReleased(_button) => {}
        // MouseEntered => {}
        // MouseExited => {}
        // MouseWheel(_amount, _phase) => {}
        // Moved(_pos) => {}
        // Resized(_size) => {}
        // Touch(_touch) => {}
        // TouchPressure(_pressure) => {}
        // HoveredFile(_path) => {}
        // DroppedFile(_path) => {}
        // HoveredFileCancelled => {}
        // Focused => {}
        // Unfocused => {}
        // Closed => {}
        _ => {}
    }
}

fn update(_app: &App, model: &mut Model, _update: Update) {
    match model.events.app_rx.try_recv() {
        Ok(message::Message::ConfigUpdate(new_cfg)) => {
            println!("New Config: {:?}", &new_cfg);
            // model.inputs.drain(..).for_each(|i| i.close());
            // model.inputs = midi::scan_inputs(new_cfg, model.events.midi_tx.clone(), model.events.audio_tx.clone());
            // println!("Midi inputs reset")
        }
        _ => ()
    }
}

fn view(app: &App, _model: &Model, frame: Frame){
    let draw = app.draw();
    draw.background().color(PURPLE);

    // We'll align to the window dimensions, but padded slightly.
    let win_rect = app.main_window().rect().pad(20.0);

    let text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.\n\nResize the window to test dynamic layout.";

    //                         L     o     r     e     m           i    p    s    u    m
    let glyph_colors = vec![BLUE, BLUE, BLUE, BLUE, BLUE, BLACK, RED, RED, RED, RED, RED];

    draw.text(text)
        .color(BLACK)
        .glyph_colors(glyph_colors)
        .font_size(24)
        .wh(win_rect.wh());

    draw.to_frame(app, &frame).unwrap();
}

