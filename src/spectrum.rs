use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;
use myalgorithm::get_freq;
use myalgorithm::get_normalized_db;
use myalgorithm::SAMPLE_RATE;
use myalgorithm::BUFFER_SZ_HALF;
use myalgorithm::BUFFER_SZ;

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
        //使用FFT库（如rustfft）计划一个正向FFT，长度为BUFFER_SZ
        let fft = self.fft_planner.plan_fft_forward(BUFFER_SZ);
        //对输入音频audio_buffer应用窗函数（如汉宁窗），减少频谱泄漏
        let mut complex_buffer = apply_window(audio_buffer);
        //执行FFT，结果存储在complex_buffer中（复数形式）
        fft.process(&mut complex_buffer);
        
        // 计算RMS和峰值用于动态范围控制
        // 计算音频的​​rms（有效值）​​，反映整体能量
        let rms = (audio_buffer.iter().map(|&x| x * x).sum::<f32>() / audio_buffer.len() as f32).sqrt();
        // 计算​​峰值​peak​，即音频样本的最大绝对值
        let peak = audio_buffer.iter().fold(0.0f32, |max, &x| max.max(x.abs()));
        // 动态范围取rms和峰值peak70%的较大者，用于后续幅度调整
        let dynamic_range = rms.max(peak * 0.7);
        

        // 修改频谱计算，使用动态范围
        let mut spectrum: Vec<f32> = complex_buffer.iter()
            .take(BUFFER_SZ_HALF)
            .enumerate()
            .filter_map(|(i, c)| {
                // 将fft的结果换算成频率
                let freq = get_freq(i);
                // 超过最大频率（采样率的一半）的不予处理
                if freq > SAMPLE_RATE / 2.0 {
                    return None;
                }

                //ERB调整​​：等效矩形带宽模型，模拟人耳对不同频率的感知带宽
                let erb = 21.4 * (0.00437 * freq + 1.0).log10();
                /*幅度计算​​：
                c.norm()获取复数幅度（即FFT结果的模）。
                除以BUFFER_SZ_HALF（FFT长度的一半）进行归一化，假设FFT结果对称。
                乘以dynamic_range调整动态范围，增强或抑制整体幅度*/
                let magnitude = c.norm() / BUFFER_SZ_HALF as f32 * dynamic_range;
                /*公式：20 * log10(magnitude)，将幅度转换为分贝（dB）。
                加1e-10避免对零取对数，确保数值稳定*/
                let db = 20.0 * (magnitude + 1e-10).log10();
                //归一化与限制​​
                let normalized_db = get_normalized_db(db).clamp(0.0, 1.2);
                //ERB加权​​：增加高频的权重，因ERB随频率增大，调整频谱形状以更符合听觉特性
                Some(normalized_db * (1.0 + erb * 0.1))
            })
            .collect();
        //填充频谱至BUFFER_SZ长度
        while spectrum.len() < BUFFER_SZ {
            spectrum.push(0.0);
        }
        //应用平滑处理（如移动平均）减少频谱波动
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

fn smooth_spectrum(spectrum: &mut Vec<f32>) {
    /*平滑处理波谱*/
    for i in 1..spectrum.len()-1 {
        spectrum[i] = spectrum[i] * 0.5 + spectrum[i - 1] * 0.25 + spectrum[i + 1] * 0.25;
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
