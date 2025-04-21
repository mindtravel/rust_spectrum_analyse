use cpal::traits::DeviceTrait;
use parking_lot::Mutex;
use ringbuf::HeapRb;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::device::AudioDeviceManager;
use crate::spectrum::SpectrumAnalyzer;

#[derive(Clone)]
pub struct AudioCapture {
    device_manager: AudioDeviceManager,
    spectrum: Arc<Mutex<Vec<f32>>>,
}

impl AudioCapture {
    pub fn new(spectrum: Arc<Mutex<Vec<f32>>>) -> Self {
        Self {
            device_manager: AudioDeviceManager::new(),
            spectrum,
        }
    }

    pub fn start_capture(&self) -> Option<cpal::Stream> {
        let device = self.device_manager.get_default_device();
        match self.get_device_config(&device) {
            Ok(config) => match self.create_audio_stream(device, config) {
                Ok(stream) => Some(stream),
                Err(e) => {
                    println!("Failed to create audio stream: {}", e);
                    None
                }
            },
            Err(e) => {
                println!("Failed to get device config: {}", e);
                None
            }
        }
    }

    pub fn switch_device(&self, index: usize) -> Option<cpal::Stream> {
        if let Some(device) = self.device_manager.get_device_by_index(index) {
            match self.get_device_config(&device) {
                Ok(config) => {
                    match self.create_audio_stream(device, config) {
                        Ok(stream) => Some(stream),
                        Err(err) => {
                            println!("Failed to create audio stream: {}", err);
                            None
                        }
                    }
                }
                Err(err) => {
                    println!("Failed to get device config: {}", err);
                    None
                }
            }
        } else {
            println!("Invalid device index");
            None
        }
    }

    pub fn print_device_list(&self) {
        self.device_manager.print_device_list();
    }

    fn get_device_config(&self, device: &cpal::Device) -> Result<cpal::SupportedStreamConfig, String> {
        println!("Trying to get config for device: {}", device.name().unwrap_or_default());
        
        let supported_configs = match device.supported_input_configs() {
            Ok(configs) => configs,
            Err(e) => return Err(format!("Failed to get supported configs: {}", e)),
        };

        let mut configs: Vec<_> = supported_configs.collect();
        if configs.is_empty() {
            return Err("Device does not support any input configurations".to_string());
        }

        // Try preferred sample rates
        let preferred_rates = [44100, 48000, 96000, 192000];
        for &rate in &preferred_rates {
            if let Some(config) = configs.iter()
                .find(|c| {
                    let min_rate = c.min_sample_rate().0;
                    let max_rate = c.max_sample_rate().0;
                    min_rate <= rate && rate <= max_rate
                })
            {
                println!("Selected sample rate: {}Hz", rate);
                return Ok(config.with_sample_rate(cpal::SampleRate(rate)));
            }
        }

        // Fall back to the lowest supported rate
        configs.sort_by_key(|c| c.min_sample_rate().0);
        if let Some(config) = configs.first() {
            let rate = config.min_sample_rate();
            println!("Using minimum sample rate: {}Hz", rate.0);
            Ok(config.with_sample_rate(rate))
        } else {
            Err("Could not find suitable audio configuration".to_string())
        }
    }

    fn create_audio_stream(
        &self,
        device: cpal::Device,
        config: cpal::SupportedStreamConfig,
    ) -> Result<cpal::Stream, String> {
        let ring = HeapRb::<f32>::new(8192);
        let (mut producer, mut consumer) = ring.split();
        let spectrum = self.spectrum.clone();
        let sample_rate = config.sample_rate().0 as f32;

        std::thread::Builder::new()
            .name("audio_processing".to_string())
            .spawn(move || {
                let mut buffer = Vec::with_capacity(2048);
                let mut last_process = Instant::now();
                let mut analyzer = SpectrumAnalyzer::new(sample_rate);
                
                loop {
                    let now = Instant::now();
                    if now.duration_since(last_process).as_millis() < 16 {
                        std::thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    
                    while consumer.len() >= 2048 {
                        buffer.clear();
                        buffer.extend(consumer.pop_iter().take(2048));
                        let spectrum_data = analyzer.compute_spectrum(&buffer);
                        *spectrum.lock() = spectrum_data;
                    }
                    
                    last_process = now;
                }
            })
            .map_err(|e| format!("Failed to spawn audio thread: {}", e))?;

        let gain = if device.name()
            .map(|n| n.to_lowercase().contains("vb-"))
            .unwrap_or(false) 
        {
            2.0
        } else {
            1.0
        };

        device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    let compensated: Vec<f32> = data.iter()
                        .map(|&x| x * gain)
                        .collect();
                    
                    if producer.push_slice(&compensated) < compensated.len() {
                        eprintln!("Buffer overflow");
                    }
                },
                |err| eprintln!("Audio stream error: {err:?}"),
                None,
            )
            .map_err(|e| format!("Failed to build input stream: {}", e))
    }
}
