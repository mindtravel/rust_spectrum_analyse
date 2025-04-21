use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, Host};

pub struct AudioDeviceManager {
    host: Host,
    devices: Vec<Device>,
}

// 手动实现 Clone，避免对 Host 的克隆要求
impl Clone for AudioDeviceManager {
    fn clone(&self) -> Self {
        // 重新创建一个新的 Host 实例
        Self {
            host: cpal::default_host(),
            devices: self.devices.clone(),
        }
    }
}

impl AudioDeviceManager {
    pub fn new() -> Self {
        let host = cpal::default_host();
        let devices = Self::enumerate_devices(&host);
        Self { host, devices }
    }

    fn enumerate_devices(host: &Host) -> Vec<Device> {
        println!("\n=== 系统音频设备列表 ===");
        let devices: Vec<_> = host.devices()
            .expect("无法枚举音频设备")
            .filter_map(|device| {
                if let Ok(name) = device.name() {
                    // 显示设备详细信息
                    println!("发现设备: {}", name);
                    if let Ok(config) = device.default_input_config() {
                        println!("  采样率: {}Hz", config.sample_rate().0);
                        println!("  声道数: {}", config.channels());
                    }
                    Some(device)
                } else {
                    None
                }
            })
            .collect();

        if devices.is_empty() {
            println!("警告: 未找到任何音频设备!");
        }
        devices
    }

    pub fn find_loopback_device(&self) -> Option<Device> {
        println!("\n=== 检测系统音频设备 ===");

        // 1. 首先尝试查找 VB-Cable 虚拟设备
        let vb_device = self.devices.iter().find(|device| {
            let name = device.name().unwrap_or_default().to_lowercase();
            name.contains("vb-audio") || 
            name.contains("cable input") ||
            name.contains("voicemeeter")
        });

        if let Some(device) = vb_device {
            println!("找到虚拟音频设备: {}", device.name().unwrap_or_default());
            return Some(device.clone());
        }

        // 2. 尝试使用 WASAPI 环回捕获
        #[cfg(target_os = "windows")]
        {
            if let Some(output) = self.host.default_output_device() {
                if let Ok(configs) = output.supported_input_configs() {
                    if configs.count() > 0 {
                        println!("使用 WASAPI 环回捕获: {}", 
                                output.name().unwrap_or_default());
                        return Some(output);
                    }
                }
            }
        }

        // 3. 查找其他回环设备
        let loopback = self.devices.iter().find(|device| {
            let name = device.name().unwrap_or_default().to_lowercase();
            name.contains("立体声混音") || 
            name.contains("stereo mix") || 
            name.contains("what u hear")
        });

        if let Some(device) = loopback {
            println!("找到系统回环设备: {}", device.name().unwrap_or_default());
            return Some(device.clone());
        }

        println!("未找到专用回环设备，使用默认输入设备");
        None
    }

    pub fn get_default_device(&self) -> Device {
        // 获取所有支持音频输入的设备
        let available_devices: Vec<_> = self.devices.iter()
            .filter(|device| {
                if let Ok(configs) = device.supported_input_configs() {
                    if configs.count() > 0 {
                        println!("设备 {} 支持音频输入", device.name().unwrap_or_default());
                        return true;
                    }
                }
                false
            })
            .cloned()
            .collect();

        // 按优先级查找设备
        available_devices.iter()
            // 1. 查找 VB-Cable
            .find(|device| {
                device.name()
                    .map(|n| n.to_lowercase().contains("vb-"))
                    .unwrap_or(false)
            })
            // 2. 查找立体声混音
            .or_else(|| {
                available_devices.iter().find(|device| {
                    device.name()
                        .map(|n| n.to_lowercase().contains("立体声混音"))
                        .unwrap_or(false)
                })
            })
            // 3. 使用第一个可用设备
            .cloned()
            .or_else(|| available_devices.first().cloned())
            .expect("未找到任何可用的音频设备")
    }

    pub fn list_devices(&self) -> Vec<(usize, String)> {
        self.devices.iter().enumerate()
            .filter_map(|(idx, device)| {
                device.name().ok().map(|name| (idx, name))
            })
            .collect()
    }

    pub fn get_device_by_index(&self, index: usize) -> Option<Device> {
        self.devices.get(index).cloned()
    }

    pub fn print_device_list(&self) {
        println!("\n=== 可用音频设备列表 ===");
        for (idx, name) in self.list_devices() {
            println!("[{}] {}", idx, name);
        }
        println!("\n输入设备编号以切换设备，输入其他内容继续使用当前设备");
    }
}
