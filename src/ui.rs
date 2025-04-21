use egui::{Align2, Color32, FontId, Pos2, Rect, Ui};

// 绘制频谱
pub fn draw_spectrum(ui: &mut Ui, spectrum: &[f32]) {
    let rect = ui.available_rect_before_wrap();
    let painter = ui.painter();
    let _clip_rect = ui.clip_rect();
    let plot_rect = rect.shrink(30.0);

    draw_background(painter, &plot_rect);
    draw_spectrum_lines(painter, &plot_rect, spectrum);
    draw_axes(painter, &plot_rect);
    draw_frequency_marks(painter, &plot_rect);
    draw_db_marks(painter, &plot_rect);
}

// 绘制背景
fn draw_background(painter: &egui::Painter, plot_rect: &Rect) {
    painter.rect_filled(*plot_rect, 0.0, Color32::from_rgb(20, 20, 20));
}

// 绘制频谱曲线
fn draw_spectrum_lines(painter: &egui::Painter, plot_rect: &Rect, spectrum: &[f32]) {
    let sample_rate = 44100.0;
    let mut points = Vec::with_capacity(spectrum.len());
    let mut colors = Vec::with_capacity(spectrum.len());

    // 只处理到20kHz的数据
    let max_freq = 20000.0;
    // let max_index = ((max_freq * 8192.0) / sample_rate) as usize;
    let max_index = 2048 as usize;

    for (i, &value) in spectrum.iter().take(max_index).enumerate() {
        // 计算当前的频率
        let freq = (i as f32 * sample_rate / 1024.0) + 1.0;

        // 统一的频率到坐标的映射函数
        let x = freq_to_x_coord(freq, plot_rect);

        let db = 20.0 * (value + 1e-10).log10();
        let db_normalized = (db + 60.0) / 60.0;
        let height = db_normalized.clamp(0.0, 1.0) * plot_rect.height() * 0.9; // 缩小高度到90%

        // 根据频段选择颜色
        let color = Color32::from_rgb(
            (255.0 * db_normalized) as u8,
            (255.0 * (1.0 - db_normalized)) as u8,
            50,
        );

        points.push(Pos2::new(x, plot_rect.bottom() - height));
        colors.push(color);
    }

    // 绘制彩色线段
    if points.len() >= 2 {
        for i in 0..points.len() - 1 {
            painter.line_segment(
                [points[i], points[i + 1]],
                egui::Stroke::new(2.0, colors[i]),
            );
        }
    }
}

// 添加统一的频率到坐标的映射函数
fn freq_to_x_coord(freq: f32, plot_rect: &Rect) -> f32 {
    let log_x = (freq.log10() - 1.0) / 3.5;  // 调整为4.0使刻度分布更均匀
    plot_rect.left() + log_x * plot_rect.width() + 60.0
}

fn get_frequency_band_color(_freq: f32, intensity: f32) -> Color32 {
    // 使用HSV颜色空间进行渐变
    let hue = 0.33 * (1.0 - (intensity * -60.0).clamp(0.0, 1.0));
    let saturation = 1.0;
    let value = 0.7 + 0.3 * intensity;

    // HSV转RGB
    let c = value * saturation;
    let x = c * (1.0 - ((hue * 6.0) % 2.0 - 1.0).abs());
    let m = value - c;

    let (r, g, b) = match (hue * 6.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    Color32::from_rgb(
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

fn draw_axes(painter: &egui::Painter, plot_rect: &Rect) {
    // 绘制横坐标轴
    painter.line_segment(
        [
            Pos2::new(plot_rect.left(), plot_rect.bottom()),
            Pos2::new(plot_rect.right(), plot_rect.bottom()),
        ],
        (1.0, Color32::LIGHT_GRAY),
    );

    // 绘制纵坐标轴
    painter.line_segment(
        [
            Pos2::new(plot_rect.left(), plot_rect.top()),
            Pos2::new(plot_rect.left(), plot_rect.bottom()),
        ],
        (1.0, Color32::LIGHT_GRAY),
    );

    // 绘制零点标记
    painter.text(
        Pos2::new(plot_rect.left() - 15.0, plot_rect.bottom() + 15.0),
        Align2::CENTER_CENTER,
        "0",
        FontId::monospace(10.0),
        Color32::LIGHT_GRAY,
    );
}

fn draw_frequency_marks(painter: &egui::Painter, plot_rect: &Rect) {
    let freq_marks = [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000];

    for &freq in &freq_marks {
        let x = freq_to_x_coord(freq as f32, plot_rect);

        // 刻度线
        painter.line_segment(
            [
                Pos2::new(x, plot_rect.bottom()),
                Pos2::new(x, plot_rect.bottom() + 5.0),
            ],
            (1.0, Color32::LIGHT_GRAY),
        );

        // 刻度标签
        painter.text(
            Pos2::new(x, plot_rect.bottom() + 8.0),
            Align2::CENTER_TOP,
            if freq >= 1000 {
                format!("{}k", freq / 1000)
            } else {
                format!("{}", freq)
            },
            FontId::monospace(10.0),
            Color32::LIGHT_GRAY,
        );
    }
}

fn draw_db_marks(painter: &egui::Painter, plot_rect: &Rect) {
    let db_marks = [-60, -50, -40, -30, -20, -10, 0];
    let plot_height = plot_rect.height() * 0.9; // 使用90%的高度

    for &db in &db_marks {
        let y = plot_rect.bottom() - ((db + 60) as f32 / 60.0) * plot_height;

        // 刻度线
        painter.line_segment(
            [
                Pos2::new(plot_rect.left() - 5.0, y),
                Pos2::new(plot_rect.left(), y),
            ],
            (1.0, Color32::LIGHT_GRAY),
        );

        // 分贝标签
        painter.text(
            Pos2::new(plot_rect.left() - 8.0, y),
            Align2::RIGHT_CENTER,
            format!("{}dB", db),
            FontId::monospace(10.0),
            Color32::LIGHT_GRAY,
        );
    }
}
