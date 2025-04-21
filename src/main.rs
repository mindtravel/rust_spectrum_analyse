mod app;
mod audio;
mod spectrum;
mod ui;

use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;
use std::io::{self, Write};
use crossbeam_channel::unbounded;
use cpal::traits::StreamTrait;
use crate::audio::AudioCapture;

// 定义设备切换命令
enum AudioCommand {
    SwitchDevice(usize),
    Quit,
}

fn main() {
    let spectrum = Arc::new(Mutex::new(vec![0.0; 2048]));
    let audio_capture = AudioCapture::new(spectrum.clone());
    
    // 显示设备列表
    audio_capture.print_device_list();
    
    // 创建命令通道
    let (cmd_tx, cmd_rx) = unbounded::<AudioCommand>();
    
    // 启动音频管理线程
    let audio_handle = std::thread::spawn(move || {
        let mut current_stream = audio_capture.start_capture()
            .and_then(|stream| stream.play().ok().map(|_| stream));
            
        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                AudioCommand::SwitchDevice(index) => {
                    // 先停止当前流
                    if let Some(stream) = current_stream.take() {
                        drop(stream);
                    }
                    
                    // 创建新流
                    if let Some(new_stream) = audio_capture.switch_device(index) {
                        if new_stream.play().is_ok() {
                            current_stream = Some(new_stream);
                            println!("成功切换到新设备");
                        }
                    }
                }
                AudioCommand::Quit => break,
            }
        }
    });

    // 启动用户输入线程
    let cmd_tx_clone = cmd_tx.clone();
    std::thread::spawn(move || {
        loop {
            print!("\n输入设备编号切换设备 (按回车继续): ");
            io::stdout().flush().unwrap();
            
            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_ok() {
                if let Ok(index) = input.trim().parse::<usize>() {
                    let _ = cmd_tx_clone.send(AudioCommand::SwitchDevice(index));
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });

    let mut options = eframe::NativeOptions::default();
    options.vsync = true;
    options.multisampling = 16;
    options.transparent = false;
    options.renderer = eframe::Renderer::Glow;
    options.initial_window_size = Some(egui::vec2(800.0, 400.0));
    options.min_window_size = Some(egui::vec2(400.0, 200.0));
    
    // 启用硬件加速
    options.hardware_acceleration = eframe::HardwareAcceleration::Preferred;
    
    eframe::run_native(
        "实时频谱分析仪",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            cc.egui_ctx.set_pixels_per_point(1.0);
            Box::new(app::SpectrumApp::new(spectrum.clone()))
        }),
    )
    .unwrap();

    // 程序退出时发送退出命令
    let _ = cmd_tx.send(AudioCommand::Quit);
    let _ = audio_handle.join();
}
