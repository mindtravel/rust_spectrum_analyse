use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;

#[derive(Copy, Clone)]
pub enum Resolution {
    Standard,
    High,
}

pub struct BandpassFilter {
    low: f32,
    high: f32,
    y1: f32,
    y2: f32,
}

impl BandpassFilter {
    fn new(low: f32, high: f32) -> Self {
        Self {
            low,
            high,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn process(&mut self, x: f32, sample_rate: f32) -> f32 {
        let dt = 1.0 / sample_rate;
        let rc_low = 1.0 / (2.0 * PI * self.high);
        let rc_high = 1.0 / (2.0 * PI * self.low);
        let alpha_low = dt / (rc_low + dt);
        let alpha_high = rc_high / (rc_high + dt);
        
        self.y1 = alpha_low * x + (1.0 - alpha_low) * self.y1;
        self.y2 = alpha_high * self.y1 + (1.0 - alpha_high) * self.y2;
        self.y1 - self.y2
    }
}

pub struct SpectrumAnalyzer {
    filters: Vec<BandpassFilter>,
    fft_planner: FftPlanner<f32>,
    resolution: Resolution,
    sample_rate: f32,
}

impl SpectrumAnalyzer {
    pub fn new(sample_rate: f32) -> Self {
        let filters = vec![
            BandpassFilter::new(0.0, 80.0),     // 低频段
            BandpassFilter::new(80.0, 1000.0),   // 中低频
            BandpassFilter::new(1000.0, 6000.0), // 中高频
            BandpassFilter::new(6000.0, 20000.0),// 高频段
        ];

        Self {
            filters,
            fft_planner: FftPlanner::new(),
            resolution: Resolution::High,
            sample_rate,
        }
    }

    pub fn compute_spectrum(&mut self, audio_buffer: &[f32]) -> Vec<f32> {
        let fft = self.fft_planner.plan_fft_forward(2048);
        let mut complex_buffer = apply_window(audio_buffer);
        fft.process(&mut complex_buffer);
        
        // 计算RMS和峰值用于动态范围控制
        let rms = (audio_buffer.iter().map(|&x| x * x).sum::<f32>() / audio_buffer.len() as f32).sqrt();
        let peak = audio_buffer.iter().fold(0.0f32, |max, &x| max.max(x.abs()));
        let dynamic_range = rms.max(peak * 0.7);
    
        // 修改频谱计算，使用动态范围
        let mut spectrum: Vec<f32> = complex_buffer.iter()
            .take(1024)
            .enumerate()
            .filter_map(|(i, c)| {
                let freq = i as f32 * 44100.0 / 1024.0 + 1.0;
                if freq > 22050.0 {
                    return None;
                }
                
                let erb = 21.4 * (0.00437 * freq + 1.0).log10();
                let magnitude = c.norm() / 1024.0 * dynamic_range;
                
                let db = 20.0 * (magnitude + 1e-10).log10();
                let normalized = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
                
                Some(normalized * (1.0 + erb * 0.1))
            })
            .collect();

        while spectrum.len() < 2048 {
            spectrum.push(0.0);
        }
    
        smooth_spectrum(&mut spectrum);
        spectrum
    }

    fn compute_band_levels(&mut self, samples: &[f32]) -> Vec<f32> {
        let mut band_levels = Vec::with_capacity(self.filters.len());
        
        for filter in &mut self.filters {
            let filtered: Vec<f32> = samples.iter()
                .map(|&x| filter.process(x, self.sample_rate))
                .collect();
            
            let rms = (filtered.iter().map(|x| x * x).sum::<f32>() / filtered.len() as f32).sqrt();
            let peak = filtered.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
            
            // 组合RMS和峰值
            let level = 0.7 * rms + 0.3 * peak;
            band_levels.push(level);
        }
        
        band_levels
    }
}

fn apply_window(audio_buffer: &[f32]) -> Vec<Complex<f32>> {
    audio_buffer
        .iter()
        .enumerate()
        .map(|(i, &x)| {
            let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / 4095.0).cos());
            Complex::new(x * window, 0.0)
        })
        .collect()
}

fn compute_magnitude_spectrum(complex_buffer: &[Complex<f32>]) -> Vec<f32> {
    let mut spectrum: Vec<f32> = complex_buffer
        .iter()
        .take(2048)
        .map(|c| {
            let magnitude = c.norm() / 2048.0;
            magnitude.powf(0.5)
        })
        .collect();

    smooth_spectrum(&mut spectrum);
    spectrum
}

fn smooth_spectrum(spectrum: &mut Vec<f32>) {
    for i in 1..spectrum.len() {
        spectrum[i] = spectrum[i] * 0.5 + spectrum[i - 1] * 0.5;
    }
}

// 添加新的滤波器实现
fn apply_bandpass(samples: &[f32], low: f32, high: f32, sample_rate: f32) -> Vec<f32> {
    // 简单的IIR滤波器实现
    let dt = 1.0 / sample_rate;
    let rc_low = 1.0 / (2.0 * PI * high);
    let rc_high = 1.0 / (2.0 * PI * low);
    let alpha_low = dt / (rc_low + dt);
    let alpha_high = rc_high / (rc_high + dt);
    
    let mut output = Vec::with_capacity(samples.len());
    let mut y1 = 0.0;
    let mut y2 = 0.0;
    
    for &x in samples {
        y1 = alpha_low * x + (1.0 - alpha_low) * y1;
        y2 = alpha_high * y1 + (1.0 - alpha_high) * y2;
        output.push(y1 - y2);
    }
    
    output
}
