use jack::{self, AudioIn, Client, ClientOptions, ProcessHandler};
use jack_sys as j;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;

use crate::spectrum::compute_spectrum;

pub struct JackAudioCapture {
    _client: jack::AsyncClient<(), JackHandler>,
    spectrum: Arc<Mutex<Vec<f32>>>,
}

struct JackHandler {
    port: jack::Port<AudioIn>,
    spectrum: Arc<Mutex<Vec<f32>>>,
    buffer: Vec<f32>,
}

impl JackAudioCapture {
    pub fn new(spectrum: Arc<Mutex<Vec<f32>>>) -> Result<Self, Box<dyn std::error::Error>> {
        let (client, _status) = Client::new("spectrum_analyzer", ClientOptions::NO_START_SERVER)?;

        println!("JACK 采样率: {}", client.sample_rate());
        
        let port = client.register_port("input", AudioIn::default())?;
        let handler = JackHandler {
            port,
            spectrum: spectrum.clone(),
            buffer: Vec::with_capacity(4096),
        };

        let active_client = client.activate_async((), handler)?;

        Ok(Self {
            _client: active_client,
            spectrum,
        })
    }

    pub fn start_capture(&self) -> Result<(), Box<dyn std::error::Error>> {
        // JACK client 已经在创建时自动启动
        std::thread::sleep(Duration::from_millis(100)); // 等待JACK启动
        Ok(())
    }
}

impl ProcessHandler for JackHandler {
    fn process(&mut self, _: &Client, ps: &jack::ProcessScope) -> jack::Control {
        let in_port = self.port.as_slice(ps);
        self.buffer.extend_from_slice(in_port);

        if self.buffer.len() >= 4096 {
            let spectrum_data = compute_spectrum(&self.buffer[..4096]);
            *self.spectrum.lock() = spectrum_data;
            self.buffer.clear();
        }

        jack::Control::Continue
    }
}
