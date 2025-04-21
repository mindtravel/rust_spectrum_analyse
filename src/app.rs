use crate::ui::draw_spectrum;
use egui;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Instant;

pub struct SpectrumApp {
    spectrum: Arc<Mutex<Vec<f32>>>,
    display_buffer: Vec<f32>,
    last_update: Instant,
    frame_buffer: Vec<f32>,    // 添加帧缓冲
    interpolation: f32,        // 添加插值因子
    frame_time: Instant,
    frame_count: u32,
}

impl SpectrumApp {
    pub fn new(spectrum: Arc<Mutex<Vec<f32>>>) -> Self {
        Self {
            spectrum,
            display_buffer: vec![0.0; 2048],
            frame_buffer: vec![0.0; 2048],
            interpolation: 0.0,
            last_update: Instant::now(),
            frame_time: Instant::now(),
            frame_count: 0,
        }
    }

    fn update_display_buffer(&mut self) {
        let spectrum = self.spectrum.lock();
        // 使用双重缓冲和插值更新
        self.frame_buffer.copy_from_slice(&self.display_buffer);
        
        for (i, &value) in spectrum.iter().enumerate() {
            // 使用指数平滑
            let target = value.max(self.display_buffer[i] * 0.95);
            self.display_buffer[i] = self.display_buffer[i] * 0.8 + target * 0.2;
        }
    }
    
    // 显示事件列表
    pub fn show_device_switcher(&self) {
        use std::io::{self, Write};
        
        print!("按回车键显示设备列表...");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            // 继续使用当前设备
        }
    }
}

impl eframe::App for SpectrumApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 帧率控制和性能监控
        self.frame_count += 1;
        let now = Instant::now();
        if now.duration_since(self.frame_time).as_secs_f32() >= 1.0 {
            println!("FPS: {}", self.frame_count);
            self.frame_count = 0;
            self.frame_time = now;
        }

        // 强制持续渲染
        ctx.request_repaint();
        
        // 优化绘制逻辑
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(0, 0, 0)))
            .show(ctx, |ui| {
                ui.ctx().request_repaint(); // 确保连续重绘
                self.update_display_buffer();
                draw_spectrum(ui, &self.display_buffer);
            });
    }
}
