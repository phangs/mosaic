use eframe::egui;
use image::{ImageBuffer, Rgba, RgbaImage};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder, Icon,
};
use std::sync::{Arc, Mutex};
use enigo::{Enigo, Keyboard, Key, Settings, Direction, Mouse, Button, Coordinate};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct AppConfig {
    autostart: bool,
    save_dir: String,
    auto_preview: bool,
}

fn default_save_dir() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let mut pictures = std::path::PathBuf::from(&home);
    pictures.push("Pictures");
    if pictures.exists() {
        pictures.to_string_lossy().to_string()
    } else {
        home
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            autostart: false,
            save_dir: default_save_dir(),
            auto_preview: false,
        }
    }
}

fn get_config_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let mut path = std::path::PathBuf::from(home);
    path.push(".config");
    path.push("screamshot");
    std::fs::create_dir_all(&path).ok();
    path.push("config.json");
    path
}

fn load_config() -> AppConfig {
    let path = get_config_path();
    if let Ok(file) = std::fs::File::open(path) {
        serde_json::from_reader(file).unwrap_or_default()
    } else {
        AppConfig::default()
    }
}

fn save_config(config: &AppConfig) {
    let path = get_config_path();
    if let Ok(file) = std::fs::File::create(path) {
        let _ = serde_json::to_writer_pretty(file, config);
    }
}

#[cfg(not(target_os = "linux"))]
fn update_autostart(_enabled: bool) {}

#[cfg(target_os = "linux")]
fn update_autostart(enabled: bool) {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let mut autostart_dir = std::path::PathBuf::from(&home);
    autostart_dir.push(".config");
    autostart_dir.push("autostart");
    std::fs::create_dir_all(&autostart_dir).ok();
    
    let mut desktop_file = autostart_dir;
    desktop_file.push("screamshot.desktop");

    if enabled {
        if let Ok(current_exe) = std::env::current_exe() {
            let current_exe_str = current_exe.to_string_lossy();
            
            let mut icon_path = std::path::PathBuf::from(&home);
            icon_path.push(".local");
            icon_path.push("share");
            icon_path.push("icons");
            icon_path.push("screamshot.png");
            
            let content = format!(
                "[Desktop Entry]\n\
                Type=Application\n\
                Name=Screamshot\n\
                Exec={}\n\
                Icon={}\n\
                Comment=Region-Based Scrolling Capture\n\
                Terminal=false\n",
                current_exe_str,
                icon_path.to_string_lossy()
            );
            let _ = std::fs::write(desktop_file, content);
        }
    } else {
        let _ = std::fs::remove_file(desktop_file);
    }
}

fn generate_icon_raw() -> (Vec<u8>, u32, u32) {
    let width = 64;
    let height = 64;
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(width, height);
    
    let cx = 31.5;
    let cy = 31.5;
    
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let px = x as f32;
        let py = y as f32;
        
        let dx = px - cx;
        let dy = py - cy;
        
        let mut r = 0.0;
        let mut g = 0.0;
        let mut b = 0.0;
        let mut a = 0.0;
        
        let mut blend = |br: f32, bg: f32, bb: f32, ba: f32| {
            if ba <= 0.0 { return; }
            let out_a = ba + a * (1.0 - ba);
            if out_a > 0.0 {
                r = (br * ba + r * a * (1.0 - ba)) / out_a;
                g = (bg * ba + g * a * (1.0 - ba)) / out_a;
                b = (bb * ba + b * a * (1.0 - ba)) / out_a;
                a = out_a;
            }
        };
        
        // 1. Camera Body/Base Frame: A sleek, dark rounded rectangle
        let rx = dx.abs() - 12.0;
        let ry = dy.abs() - 9.0;
        let dist_box = if rx > 0.0 && ry > 0.0 {
            (rx*rx + ry*ry).sqrt() - 6.0
        } else if rx > 0.0 {
            rx - 6.0
        } else if ry > 0.0 {
            ry - 6.0
        } else {
            rx.max(ry) - 6.0
        };
        
        if dist_box < 1.0 {
            let antialias = (1.0 - dist_box).clamp(0.0, 1.0);
            let body_a = 0.85 * antialias;
            let grad = (dy + 15.0) / 30.0;
            let body_r = 30.0 + grad * 20.0;
            let body_g = 25.0 + grad * 15.0;
            let body_b = 45.0 + grad * 25.0;
            blend(body_r / 255.0, body_g / 255.0, body_b / 255.0, body_a);
        }
        
        // 2. Camera Flash/LED indicator
        let dist_led = ((dx - 10.0).powi(2) + (dy + 7.0).powi(2)).sqrt();
        if dist_led < 2.5 {
            let antialias = (2.5 - dist_led).clamp(0.0, 1.0);
            blend(1.0, 0.2, 0.4, 0.9 * antialias);
        }
        
        // 3. Camera Lens Base Ring
        let l_cx = -2.0;
        let l_cy = 1.0;
        let dist_lens = ((dx - l_cx).powi(2) + (dy - l_cy).powi(2)).sqrt();
        if dist_lens < 11.5 {
            let antialias = (11.5 - dist_lens).clamp(0.0, 1.0).min((dist_lens - 8.5).clamp(0.0, 1.0));
            blend(0.05, 0.05, 0.1, 0.9 * antialias);
        }
        
        // 4. Glowing Neon Lens Ring (Gradient from Cyan to Purple)
        if dist_lens < 10.0 && dist_lens > 8.0 {
            let antialias = (1.0 - (dist_lens - 9.0).abs()).clamp(0.0, 1.0);
            let angle = dy.atan2(dx);
            let t = (angle + std::f32::consts::PI) / (2.0 * std::f32::consts::PI);
            let lr = 0.0 + t * 0.6;
            let lg = 0.9 - t * 0.9;
            let lb = 0.95 + t * 0.05;
            blend(lr, lg, lb, 0.95 * antialias);
        }
        
        // 5. Deep glossy lens glass
        if dist_lens < 8.0 {
            let antialias = (8.0 - dist_lens).clamp(0.0, 1.0);
            let glass_t = ((dx - l_cx + 6.0) + (dy - l_cy + 6.0)) / 20.0;
            let gr = 0.05 + glass_t * 0.15;
            let gg = 0.02 + glass_t * 0.1;
            let gb = 0.12 + glass_t * 0.3;
            blend(gr, gg, gb, 0.9 * antialias);
        }
        
        // 6. Lens Reflection Flare
        let ref_cx = l_cx - 3.0;
        let ref_cy = l_cy - 3.0;
        let dist_ref = ((dx - ref_cx).powi(2) + (dy - ref_cy).powi(2)).sqrt();
        if dist_ref < 2.2 {
            let antialias = (2.2 - dist_ref).clamp(0.0, 1.0);
            blend(1.0, 1.0, 1.0, 0.85 * antialias);
        }
        
        // 7. Futuristic Crop brackets at the 4 corners
        let adx = dx.abs();
        let ady = dy.abs();
        if adx >= 20.0 && adx <= 26.0 && ady >= 20.0 && ady <= 26.0 {
            let on_edge_x = (adx - 26.0).abs() < 1.5;
            let on_edge_y = (ady - 26.0).abs() < 1.5;
            let is_corner_bend = (on_edge_x && ady >= 20.0) || (on_edge_y && adx >= 20.0);
            
            if is_corner_bend {
                let dist_bracket = if on_edge_x {
                    (adx - 26.0).abs()
                } else {
                    (ady - 26.0).abs()
                };
                let antialias = (1.5 - dist_bracket).clamp(0.0, 1.0);
                blend(0.0, 0.9, 1.0, 0.95 * antialias);
            }
        }
        
        *pixel = Rgba([
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
            (a * 255.0) as u8,
        ]);
    }
    
    (img.into_raw(), width, height)
}

fn generate_icon() -> Icon {
    let (rgba, width, height) = generate_icon_raw();
    Icon::from_rgba(rgba, width, height).expect("Failed to create icon")
}

fn generate_egui_icon() -> egui::IconData {
    let (rgba, width, height) = generate_icon_raw();
    egui::IconData { rgba, width, height }
}

fn find_overlap(img1: &RgbaImage, img2: &RgbaImage) -> u32 {
    let (width, height) = img1.dimensions();
    let mut best_overlap = 0;
    let mut min_diff = u64::MAX;

    let min_o = height / 10;
    
    let start_x = width / 10;
    let end_x = width * 9 / 10;
    let step_x = 4;
    let step_y = 2;

    for o in (min_o..=height).rev() {
        let mut diff: u64 = 0;
        let mut count = 0;
        let mut early_exit = false;
        
        for y in (0..o).step_by(step_y as usize) {
            for x in (start_x..end_x).step_by(step_x as usize) {
                let p1 = img1.get_pixel(x, height - o + y);
                let p2 = img2.get_pixel(x, y);
                
                let r_diff = (p1[0] as i32 - p2[0] as i32).abs() as u64;
                let g_diff = (p1[1] as i32 - p2[1] as i32).abs() as u64;
                let b_diff = (p1[2] as i32 - p2[2] as i32).abs() as u64;
                
                diff += r_diff + g_diff + b_diff;
                count += 1;
            }
            
            if count > 500 {
                let avg = diff / count as u64;
                if min_diff != u64::MAX && avg > min_diff + 20 {
                    early_exit = true;
                    break;
                }
            }
        }
        
        if !early_exit && count > 0 {
            let avg_diff = diff / count as u64;
            if avg_diff < min_diff {
                min_diff = avg_diff;
                best_overlap = o;
                
                if min_diff == 0 {
                    break;
                }
            }
        }
    }
    
    if min_diff < 30 {
        best_overlap
    } else {
        0
    }
}

fn stitch_frames(frames: Vec<RgbaImage>) -> RgbaImage {
    if frames.is_empty() {
        return RgbaImage::new(0, 0);
    }
    if frames.len() == 1 {
        return frames[0].clone();
    }
    
    let (width, height) = frames[0].dimensions();
    
    let mut offsets = vec![0];
    let mut total_height = height;
    
    for i in 1..frames.len() {
        let overlap = find_overlap(&frames[i-1], &frames[i]);
        let new_height = height - overlap;
        
        if new_height < 5 {
            offsets.push(0);
        } else {
            offsets.push(new_height);
            total_height += new_height;
        }
    }
    
    let mut result = RgbaImage::new(width, total_height);
    image::imageops::replace(&mut result, &frames[0], 0, 0);
    
    let mut current_y = height as i64;
    for i in 1..frames.len() {
        if offsets[i] == 0 {
            continue;
        }
        
        let overlap = height - offsets[i];
        let new_part = image::imageops::crop_imm(&frames[i], 0, overlap, width, offsets[i]).to_image();
        image::imageops::replace(&mut result, &new_part, 0, current_y);
        current_y += offsets[i] as i64;
    }
    
    result
}

fn find_overlap_horizontal(img1: &RgbaImage, img2: &RgbaImage) -> u32 {
    let (width, height) = img1.dimensions();
    let mut best_overlap = 0;
    let mut min_diff = u64::MAX;

    let min_o = width / 10;
    
    let start_y = height / 10;
    let end_y = height * 9 / 10;
    let step_y = 4;
    let step_x = 2;

    for o in (min_o..=width).rev() {
        let mut diff: u64 = 0;
        let mut count = 0;
        let mut early_exit = false;
        
        for x in (0..o).step_by(step_x as usize) {
            for y in (start_y..end_y).step_by(step_y as usize) {
                let p1 = img1.get_pixel(width - o + x, y);
                let p2 = img2.get_pixel(x, y);
                
                let r_diff = (p1[0] as i32 - p2[0] as i32).abs() as u64;
                let g_diff = (p1[1] as i32 - p2[1] as i32).abs() as u64;
                let b_diff = (p1[2] as i32 - p2[2] as i32).abs() as u64;
                
                diff += r_diff + g_diff + b_diff;
                count += 1;
            }
            
            if count > 500 {
                let avg = diff / count as u64;
                if min_diff != u64::MAX && avg > min_diff + 20 {
                    early_exit = true;
                    break;
                }
            }
        }
        
        if !early_exit && count > 0 {
            let avg_diff = diff / count as u64;
            if avg_diff < min_diff {
                min_diff = avg_diff;
                best_overlap = o;
                
                if min_diff == 0 {
                    break;
                }
            }
        }
    }
    
    if min_diff < 30 {
        best_overlap
    } else {
        0
    }
}

fn stitch_frames_horizontal(frames: Vec<RgbaImage>) -> RgbaImage {
    if frames.is_empty() {
        return RgbaImage::new(0, 0);
    }
    if frames.len() == 1 {
        return frames[0].clone();
    }
    
    let (width, height) = frames[0].dimensions();
    
    let mut offsets = vec![0];
    let mut total_width = width;
    
    for i in 1..frames.len() {
        let overlap = find_overlap_horizontal(&frames[i-1], &frames[i]);
        let new_width = width - overlap;
        
        if new_width < 5 {
            offsets.push(0);
        } else {
            offsets.push(new_width);
            total_width += new_width;
        }
    }
    
    let mut result = RgbaImage::new(total_width, height);
    image::imageops::replace(&mut result, &frames[0], 0, 0);
    
    let mut current_x = width as i64;
    for i in 1..frames.len() {
        if offsets[i] == 0 {
            continue;
        }
        
        let overlap = width - offsets[i];
        let new_part = image::imageops::crop_imm(&frames[i], overlap, 0, offsets[i], height).to_image();
        image::imageops::replace(&mut result, &new_part, current_x, 0);
        current_x += offsets[i] as i64;
    }
    
    result
}

#[derive(Debug, Clone, Copy)]
enum ButtonIcon {
    DownArrow,
    RightArrow,
    Checkmark,
    Cross,
}

fn premium_circular_button(
    ui: &mut egui::Ui,
    icon: ButtonIcon,
    bg_color: egui::Color32,
    size: f32,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(size, size),
        egui::Sense::click(),
    );
    
    let is_hovered = response.hovered();
    let is_clicked = response.clicked();
    
    let current_bg = if is_clicked {
        bg_color.gamma_multiply(0.7)
    } else if is_hovered {
        bg_color.gamma_multiply(0.9)
    } else {
        bg_color.gamma_multiply(0.8)
    };
    
    ui.painter().circle_filled(rect.center(), size / 2.0, current_bg);
    ui.painter().circle_stroke(
        rect.center(),
        size / 2.0,
        egui::Stroke::new(1.5, egui::Color32::from_white_alpha(150)),
    );
    
    let center = rect.center();
    let half_w = size * 0.22;
    let stroke = egui::Stroke::new(2.5, egui::Color32::WHITE);
    
    match icon {
        ButtonIcon::DownArrow => {
            ui.painter().line_segment(
                [center - egui::vec2(0.0, half_w * 0.8), center + egui::vec2(0.0, half_w * 0.6)],
                stroke,
            );
            ui.painter().line_segment(
                [center + egui::vec2(0.0, half_w * 0.6), center + egui::vec2(-half_w * 0.5, half_w * 0.1)],
                stroke,
            );
            ui.painter().line_segment(
                [center + egui::vec2(0.0, half_w * 0.6), center + egui::vec2(half_w * 0.5, half_w * 0.1)],
                stroke,
            );
        }
        ButtonIcon::RightArrow => {
            ui.painter().line_segment(
                [center - egui::vec2(half_w * 0.8, 0.0), center + egui::vec2(half_w * 0.6, 0.0)],
                stroke,
            );
            ui.painter().line_segment(
                [center + egui::vec2(half_w * 0.6, 0.0), center + egui::vec2(half_w * 0.1, -half_w * 0.5)],
                stroke,
            );
            ui.painter().line_segment(
                [center + egui::vec2(half_w * 0.6, 0.0), center + egui::vec2(half_w * 0.1, half_w * 0.5)],
                stroke,
            );
        }
        ButtonIcon::Checkmark => {
            let p1 = center + egui::vec2(-half_w * 0.7, -half_w * 0.1);
            let p2 = center + egui::vec2(-half_w * 0.1, half_w * 0.5);
            let p3 = center + egui::vec2(half_w * 0.7, -half_w * 0.5);
            ui.painter().line_segment([p1, p2], stroke);
            ui.painter().line_segment([p2, p3], stroke);
        }
        ButtonIcon::Cross => {
            let offset = half_w * 0.6;
            ui.painter().line_segment(
                [center + egui::vec2(-offset, -offset), center + egui::vec2(offset, offset)],
                stroke,
            );
            ui.painter().line_segment(
                [center + egui::vec2(offset, -offset), center + egui::vec2(-offset, offset)],
                stroke,
            );
        }
    }
    
    is_clicked
}


fn save_and_clipboard(img: RgbaImage, prefix: &str, config: &AppConfig) {
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("{}_{}.png", prefix, timestamp);
    
    let mut path = std::path::PathBuf::from(&config.save_dir);
    if !path.exists() {
        let _ = std::fs::create_dir_all(&path);
    }
    path.push(&filename);
    let path_str = path.to_string_lossy().to_string();
    
    if let Err(e) = img.save(&path) {
        eprintln!("Failed to save image: {}", e);
        return;
    }
    
    let mut copied_to_clipboard = false;
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => {
            let (c_width, c_height) = img.dimensions();
            let image_data = arboard::ImageData {
                width: c_width as usize,
                height: c_height as usize,
                bytes: std::borrow::Cow::Owned(img.into_raw()),
            };
            if let Err(e) = clipboard.set_image(image_data) {
                eprintln!("Failed to copy to clipboard: {}", e);
            } else {
                copied_to_clipboard = true;
                println!("Saved screenshot to {} and copied to clipboard!", path_str);
            }
        }
        Err(e) => {
            eprintln!("Failed to initialize clipboard: {}", e);
            println!("Saved screenshot to {}", path_str);
        }
    }

    if config.auto_preview {
        let _ = std::process::Command::new("xdg-open")
            .arg(&path_str)
            .spawn();
    }

    let summary = if prefix.starts_with("scroll") {
        "Scrolling Capture Done"
    } else {
        "Screen Capture Done"
    };

    let body = if copied_to_clipboard {
        format!("Image saved to {} and copied to clipboard!", filename)
    } else {
        format!("Image saved to {}", filename)
    };

    let _ = notify_rust::Notification::new()
        .summary(summary)
        .body(&body)
        .appname("Screamshot")
        .icon("camera-photo")
        .show();
}

#[derive(PartialEq, Clone, Copy)]
enum AppState {
    Hidden,
    SelectingRegion,
    SelectingScrollRegion,
    CapturingScroll,
    EditingSettings,
}

fn get_desktop_bounds() -> (i32, i32, i32, i32) {
    if let Ok(monitors) = xcap::Monitor::all() {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        for m in &monitors {
            let mx = m.x().unwrap_or(0);
            let my = m.y().unwrap_or(0);
            let mw = m.width().unwrap_or(0) as i32;
            let mh = m.height().unwrap_or(0) as i32;
            min_x = min_x.min(mx);
            min_y = min_y.min(my);
            max_x = max_x.max(mx + mw);
            max_y = max_y.max(my + mh);
        }
        if min_x != i32::MAX {
            return (min_x, min_y, max_x - min_x, max_y - min_y);
        }
    }
    (0, 0, 1920, 1080)
}

fn get_monitor_from_point(x: i32, y: i32) -> Option<xcap::Monitor> {
    xcap::Monitor::from_point(x, y).ok()
        .or_else(|| {
            xcap::Monitor::all().ok()?.first().cloned()
        })
}

fn get_monitor_and_crop_coords(
    start_x_relative: f32,
    start_y_relative: f32,
    rect_min_x_relative: f32,
    rect_min_y_relative: f32,
) -> Option<(i32, i32, u32, u32)> {
    let (desktop_min_x, desktop_min_y, _, _) = get_desktop_bounds();
    let global_start_x = desktop_min_x + start_x_relative as i32;
    let global_start_y = desktop_min_y + start_y_relative as i32;
    
    let monitor = get_monitor_from_point(global_start_x, global_start_y)?;
        
    let global_rect_x = desktop_min_x + rect_min_x_relative as i32;
    let global_rect_y = desktop_min_y + rect_min_y_relative as i32;
    
    let crop_x = (global_rect_x - monitor.x().unwrap_or(0)).max(0) as u32;
    let crop_y = (global_rect_y - monitor.y().unwrap_or(0)).max(0) as u32;
    
    Some((global_start_x, global_start_y, crop_x, crop_y))
}

fn show_overlay(ctx: &egui::Context) {
    let (min_x, min_y, width, height) = get_desktop_bounds();
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(min_x as f32, min_y as f32)));
    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(width as f32, height as f32)));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
}

struct ScreamshotApp {
    tray_icon: Option<TrayIcon>,
    menu_channel: tray_icon::menu::MenuEventReceiver,
    state: AppState,
    
    capture_region_i: MenuItem,
    capture_scrolling_i: MenuItem,
    settings_i: MenuItem,
    quit_i: MenuItem,
    
    selection_start: Option<egui::Pos2>,
    selection_current: Option<egui::Pos2>,
    
    scroll_rect: Option<egui::Rect>,
    scroll_frames: Arc<Mutex<Vec<RgbaImage>>>,
    scroll_horizontal: bool,
    config: AppConfig,
}

impl ScreamshotApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let tray_menu = Menu::new();
        let capture_region_i = MenuItem::new("Capture Region", true, None);
        let capture_scrolling_i = MenuItem::new("Capture Scrolling Region", true, None);
        let settings_i = MenuItem::new("Settings", true, None);
        let quit_i = MenuItem::new("Quit", true, None);

        tray_menu.append_items(&[
            &capture_region_i,
            &capture_scrolling_i,
            &settings_i,
            &PredefinedMenuItem::separator(),
            &quit_i,
        ]).unwrap();

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("Screamshot")
            .with_icon(generate_icon())
            .build()
            .unwrap();

        let config = load_config();

        Self {
            tray_icon: Some(tray_icon),
            menu_channel: MenuEvent::receiver().clone(),
            state: AppState::Hidden,
            capture_region_i,
            capture_scrolling_i,
            settings_i,
            quit_i,
            selection_start: None,
            selection_current: None,
            scroll_rect: None,
            scroll_frames: Arc::new(Mutex::new(Vec::new())),
            scroll_horizontal: false,
            config,
        }
    }
}

impl eframe::App for ScreamshotApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {}

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(target_os = "linux")]
        while gtk::glib::MainContext::default().pending() {
            gtk::glib::MainContext::default().iteration(false);
        }

        if let Ok(event) = self.menu_channel.try_recv() {
            if event.id == self.quit_i.id() {
                self.tray_icon.take();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else if event.id == self.capture_region_i.id() {
                self.state = AppState::SelectingRegion;
                show_overlay(ctx);
            } else if event.id == self.capture_scrolling_i.id() {
                self.state = AppState::SelectingScrollRegion;
                show_overlay(ctx);
            } else if event.id == self.settings_i.id() {
                self.state = AppState::EditingSettings;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(450.0, 250.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }

        if self.state == AppState::EditingSettings {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Screamshot Settings");
                ui.separator();
                ui.add_space(5.0);

                if ui.checkbox(&mut self.config.autostart, "Start Screamshot on System Startup").changed() {
                    update_autostart(self.config.autostart);
                }
                
                ui.add_space(5.0);
                ui.checkbox(&mut self.config.auto_preview, "Automatically preview captured images");
                
                ui.add_space(10.0);
                ui.label("Default Save Directory:");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.config.save_dir);
                    if ui.button("Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.config.save_dir = path.to_string_lossy().to_string();
                        }
                    }
                });

                ui.add_space(15.0);
                ui.horizontal(|ui| {
                    if ui.button("Save Settings").clicked() {
                        save_config(&self.config);
                        self.state = AppState::Hidden;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    }
                    if ui.button("Cancel").clicked() {
                        self.config = load_config(); // revert
                        self.state = AppState::Hidden;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    }
                });
            });
            
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return;
        }

        if self.state == AppState::CapturingScroll {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.fill(egui::Color32::TRANSPARENT))
                .show(ctx, |ui| {
                    if let Some(rect) = self.scroll_rect {
                        let screen_rect = ctx.viewport_rect();
                        
                        // 1. Draw selection region outline (dashed/cyan border)
                        ui.painter().rect_stroke(
                            rect, 
                            0.0, 
                            egui::Stroke::new(2.5, egui::Color32::from_rgb(0, 255, 255)), 
                            egui::StrokeKind::Inside
                        );
                        
                        // Draw a subtle translucent inner fill
                        ui.painter().rect_filled(
                            rect,
                            0.0,
                            egui::Color32::from_rgba_unmultiplied(0, 255, 255, 10)
                        );

                        // 2. Draw dark overlays covering everything OUTSIDE the selected rect
                        // Top
                        ui.painter().rect_filled(
                            egui::Rect::from_min_max(screen_rect.min, egui::pos2(screen_rect.max.x, rect.min.y)),
                            0.0,
                            egui::Color32::from_black_alpha(150),
                        );
                        // Bottom
                        ui.painter().rect_filled(
                            egui::Rect::from_min_max(egui::pos2(screen_rect.min.x, rect.max.y), screen_rect.max),
                            0.0,
                            egui::Color32::from_black_alpha(150),
                        );
                        // Left
                        ui.painter().rect_filled(
                            egui::Rect::from_min_max(egui::pos2(screen_rect.min.x, rect.min.y), egui::pos2(rect.min.x, rect.max.y)),
                            0.0,
                            egui::Color32::from_black_alpha(150),
                        );
                        // Right
                        ui.painter().rect_filled(
                            egui::Rect::from_min_max(egui::pos2(rect.max.x, rect.min.y), egui::pos2(screen_rect.max.x, rect.max.y)),
                            0.0,
                            egui::Color32::from_black_alpha(150),
                        );

                        // Render frame counter badge above the region
                        let num_frames = self.scroll_frames.lock().unwrap().len();
                        let badge_pos = egui::pos2(rect.center().x - 60.0, rect.min.y + 15.0);
                        egui::Area::new(egui::Id::new("scroll_badge"))
                            .fixed_pos(badge_pos)
                            .order(egui::Order::Foreground)
                            .show(ctx, |ui| {
                                egui::Frame::NONE
                                    .fill(egui::Color32::from_black_alpha(200))
                                    .corner_radius(8)
                                    .inner_margin(egui::Margin::symmetric(10, 6))
                                    .show(ui, |ui| {
                                        ui.colored_label(
                                            egui::Color32::WHITE,
                                            format!("Captured: {} frames", num_frames)
                                        );
                                    });
                            });

                        // Calculate constrained button positions
                        let margin = 70.0;
                        
                        // Down Arrow Button (Scroll Down)
                        let mut down_pos = egui::pos2(rect.center().x - 25.0, rect.max.y + 15.0);
                        if down_pos.y > screen_rect.max.y - margin {
                            down_pos.y = rect.max.y - 65.0;
                        }
                        
                        egui::Area::new(egui::Id::new("btn_down"))
                            .fixed_pos(down_pos)
                            .order(egui::Order::Foreground)
                            .show(ctx, |ui| {
                                 if premium_circular_button(
                                     ui,
                                     ButtonIcon::DownArrow,
                                     egui::Color32::from_rgb(0, 120, 255),
                                     50.0
                                 ) {
                                    self.scroll_horizontal = false;
                                    let frames_arc = Arc::clone(&self.scroll_frames);
                                    let ctx_clone = ctx.clone();
                                    
                                    let capture_info = get_monitor_and_crop_coords(
                                        rect.min.x,
                                        rect.min.y,
                                        rect.min.x,
                                        rect.min.y,
                                    );
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                                     
                                    std::thread::spawn(move || {
                                        std::thread::sleep(std::time::Duration::from_millis(200));
                                         
                                        if let Some((global_start_x, global_start_y, min_x, min_y)) = capture_info {
                                            if let Some(monitor) = get_monitor_from_point(global_start_x, global_start_y) {
                                                let abs_x = monitor.x().unwrap_or(0) + min_x as i32;
                                                let abs_y = monitor.y().unwrap_or(0) + min_y as i32;
                                                
                                                if let Ok(mut enigo) = Enigo::new(&Settings::default()) {
                                                    let _ = enigo.move_mouse(abs_x + 5, abs_y + 5, Coordinate::Abs);
                                                    let _ = enigo.button(Button::Left, Direction::Click);
                                                    std::thread::sleep(std::time::Duration::from_millis(50));
                                                    let _ = enigo.move_mouse(down_pos.x as i32, down_pos.y as i32, Coordinate::Abs);
                                                    std::thread::sleep(std::time::Duration::from_millis(50));
                                                    let _ = enigo.key(Key::PageDown, Direction::Click);
                                                }
                                            }
                                        }
                                         
                                        std::thread::sleep(std::time::Duration::from_millis(400));
                                         
                                        if let Some((global_start_x, global_start_y, min_x, min_y)) = capture_info {
                                            if let Some(monitor) = get_monitor_from_point(global_start_x, global_start_y) {
                                                if let Ok(mut img) = monitor.capture_image() {
                                                    let width = rect.width() as u32;
                                                    let height = rect.height() as u32;
                                                    if min_x + width <= img.width() && min_y + height <= img.height() {
                                                        let cropped = image::imageops::crop(&mut img, min_x, min_y, width, height).to_image();
                                                        let mut frames = frames_arc.lock().unwrap();
                                                        frames.push(cropped);
                                                    }
                                                }
                                            }
                                        }
                                         
                                        show_overlay(&ctx_clone);
                                    });
                                }
                            });

                        // Right Arrow Button (Scroll Right)
                        let mut right_pos = egui::pos2(rect.max.x + 15.0, rect.center().y - 25.0);
                        if right_pos.x > screen_rect.max.x - margin {
                            right_pos.x = rect.max.x - 65.0;
                        }
                        
                        egui::Area::new(egui::Id::new("btn_right"))
                            .fixed_pos(right_pos)
                            .order(egui::Order::Foreground)
                            .show(ctx, |ui| {
                                 if premium_circular_button(
                                     ui,
                                     ButtonIcon::RightArrow,
                                     egui::Color32::from_rgb(140, 0, 255),
                                     50.0
                                 ) {
                                    self.scroll_horizontal = true;
                                    let frames_arc = Arc::clone(&self.scroll_frames);
                                    let ctx_clone = ctx.clone();
                                    
                                    let capture_info = get_monitor_and_crop_coords(
                                        rect.min.x,
                                        rect.min.y,
                                        rect.min.x,
                                        rect.min.y,
                                    );
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                                     
                                    std::thread::spawn(move || {
                                        std::thread::sleep(std::time::Duration::from_millis(200));
                                         
                                        if let Some((global_start_x, global_start_y, min_x, min_y)) = capture_info {
                                            if let Some(monitor) = get_monitor_from_point(global_start_x, global_start_y) {
                                                let abs_x = monitor.x().unwrap_or(0) + min_x as i32;
                                                let abs_y = monitor.y().unwrap_or(0) + min_y as i32;
                                                
                                                if let Ok(mut enigo) = Enigo::new(&Settings::default()) {
                                                    let _ = enigo.move_mouse(abs_x + 5, abs_y + 5, Coordinate::Abs);
                                                    let _ = enigo.button(Button::Left, Direction::Click);
                                                    std::thread::sleep(std::time::Duration::from_millis(50));
                                                    let _ = enigo.move_mouse(right_pos.x as i32, right_pos.y as i32, Coordinate::Abs);
                                                    std::thread::sleep(std::time::Duration::from_millis(50));
                                                    for _ in 0..6 {
                                                        let _ = enigo.key(Key::RightArrow, Direction::Click);
                                                        std::thread::sleep(std::time::Duration::from_millis(30));
                                                    }
                                                }
                                            }
                                        }
                                         
                                        std::thread::sleep(std::time::Duration::from_millis(400));
                                         
                                        if let Some((global_start_x, global_start_y, min_x, min_y)) = capture_info {
                                            if let Some(monitor) = get_monitor_from_point(global_start_x, global_start_y) {
                                                if let Ok(mut img) = monitor.capture_image() {
                                                    let width = rect.width() as u32;
                                                    let height = rect.height() as u32;
                                                    if min_x + width <= img.width() && min_y + height <= img.height() {
                                                        let cropped = image::imageops::crop(&mut img, min_x, min_y, width, height).to_image();
                                                        let mut frames = frames_arc.lock().unwrap();
                                                        frames.push(cropped);
                                                    }
                                                }
                                            }
                                        }
                                         
                                        show_overlay(&ctx_clone);
                                    });
                                }
                            });

                        // Save/Finish Button (Checkmark)
                        let mut finish_pos = egui::pos2(rect.max.x - 60.0, rect.min.y - 65.0);
                        if finish_pos.y < 10.0 {
                            finish_pos.y = rect.min.y + 15.0;
                        }
                        
                        egui::Area::new(egui::Id::new("btn_finish"))
                            .fixed_pos(finish_pos)
                            .order(egui::Order::Foreground)
                            .show(ctx, |ui| {
                                 if premium_circular_button(
                                     ui,
                                     ButtonIcon::Checkmark,
                                     egui::Color32::from_rgb(0, 200, 80),
                                     44.0
                                 ) {
                                    let frames = {
                                        let mut frames_guard = self.scroll_frames.lock().unwrap();
                                        std::mem::take(&mut *frames_guard)
                                    };
                                    
                                    let cfg = self.config.clone();
                                    let is_horizontal = self.scroll_horizontal;
                                    
                                    if !frames.is_empty() {
                                        std::thread::spawn(move || {
                                            println!("Stitching {} frames...", frames.len());
                                            if is_horizontal {
                                                let stitched = stitch_frames_horizontal(frames);
                                                save_and_clipboard(stitched, "scroll_horizontal", &cfg);
                                            } else {
                                                let stitched = stitch_frames(frames);
                                                save_and_clipboard(stitched, "scroll", &cfg);
                                            }
                                        });
                                    }
                                    
                                    self.state = AppState::Hidden;
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                                }
                            });

                        // Cancel Button (Cross)
                        let mut cancel_pos = egui::pos2(rect.min.x, rect.min.y - 65.0);
                        if cancel_pos.y < 10.0 {
                            cancel_pos.y = rect.min.y + 15.0;
                        }
                        
                        egui::Area::new(egui::Id::new("btn_cancel"))
                            .fixed_pos(cancel_pos)
                            .order(egui::Order::Foreground)
                            .show(ctx, |ui| {
                                 if premium_circular_button(
                                     ui,
                                     ButtonIcon::Cross,
                                     egui::Color32::from_rgb(220, 40, 40),
                                     44.0
                                 ) {
                                    self.scroll_frames.lock().unwrap().clear();
                                    self.state = AppState::Hidden;
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                                }
                            });
                    }
                });
            
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
            return;
        }

        if self.state == AppState::Hidden {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return;
        }

        if self.state == AppState::SelectingRegion || self.state == AppState::SelectingScrollRegion {
            egui::Area::new(egui::Id::new("overlay"))
                .order(egui::Order::Foreground)
                .fixed_pos(egui::pos2(0.0, 0.0))
                .interactable(true)
                .show(ctx, |ui| {
                    let screen_rect = ctx.viewport_rect();
                    let response = ui.allocate_rect(screen_rect, egui::Sense::drag());

                    ui.painter().rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(150));

                    if response.drag_started() {
                        self.selection_start = response.interact_pointer_pos();
                    }
                    if response.dragged() {
                        self.selection_current = response.interact_pointer_pos();
                    }

                    if let (Some(start), Some(current)) = (self.selection_start, self.selection_current) {
                        let rect = egui::Rect::from_two_pos(start, current);
                        ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(2.0, egui::Color32::WHITE), egui::StrokeKind::Inside);
                    }

                    if response.drag_stopped() {
                        if let (Some(start), Some(end)) = (self.selection_start, self.selection_current) {
                            let width = (start.x - end.x).abs() as u32;
                            let height = (start.y - end.y).abs() as u32;

                            let state_was = self.state;
                            
                            self.selection_start = None;
                            self.selection_current = None;
                            
                            if width > 0 && height > 0 {
                                if state_was == AppState::SelectingRegion {
                                    self.state = AppState::Hidden;
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                                    let cfg = self.config.clone();
                                    let capture_info = get_monitor_and_crop_coords(
                                        start.x,
                                        start.y,
                                        start.x.min(end.x),
                                        start.y.min(end.y),
                                    );
                                    std::thread::spawn(move || {
                                        std::thread::sleep(std::time::Duration::from_millis(200));
                                        if let Some((global_start_x, global_start_y, min_x, min_y)) = capture_info {
                                            if let Some(monitor) = get_monitor_from_point(global_start_x, global_start_y) {
                                                if let Ok(mut img) = monitor.capture_image() {
                                                    if min_x + width <= img.width() && min_y + height <= img.height() {
                                                        let cropped = image::imageops::crop(&mut img, min_x, min_y, width, height).to_image();
                                                        save_and_clipboard(cropped, "screenshot", &cfg);
                                                    } else {
                                                        println!("Selection out of bounds");
                                                    }
                                                }
                                            }
                                        }
                                    });
                                } else if state_was == AppState::SelectingScrollRegion {
                                    let rect = egui::Rect::from_two_pos(start, end);
                                    self.scroll_rect = Some(rect);
                                    
                                    let frames_arc = Arc::clone(&self.scroll_frames);
                                    frames_arc.lock().unwrap().clear();
                                    
                                    self.state = AppState::CapturingScroll;
                                    
                                    let capture_info = get_monitor_and_crop_coords(
                                        start.x,
                                        start.y,
                                        start.x.min(end.x),
                                        start.y.min(end.y),
                                    );
                                    let ctx_clone = ctx.clone();
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                                    
                                    // Capture first frame in background
                                    std::thread::spawn(move || {
                                        std::thread::sleep(std::time::Duration::from_millis(300));
                                        if let Some((global_start_x, global_start_y, min_x, min_y)) = capture_info {
                                            if let Some(monitor) = get_monitor_from_point(global_start_x, global_start_y) {
                                                if let Ok(mut img) = monitor.capture_image() {
                                                    if min_x + width <= img.width() && min_y + height <= img.height() {
                                                        let cropped = image::imageops::crop(&mut img, min_x, min_y, width, height).to_image();
                                                        frames_arc.lock().unwrap().push(cropped);
                                                    }
                                                }
                                            }
                                        }
                                        show_overlay(&ctx_clone);
                                    });
                                }
                            } else {
                                self.state = AppState::Hidden;
                                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                            }
                        }
                    }
                    
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.state = AppState::Hidden;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                        self.selection_start = None;
                        self.selection_current = None;
                    }
                });
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn ensure_desktop_entry() {}

#[cfg(target_os = "linux")]
fn ensure_desktop_entry() {
    if let Ok(current_exe) = std::env::current_exe() {
        let current_exe_str = current_exe.to_string_lossy();
        
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        
        // Save the premium icon to ~/.local/share/icons/screamshot.png
        let mut icon_dir = std::path::PathBuf::from(&home);
        icon_dir.push(".local");
        icon_dir.push("share");
        icon_dir.push("icons");
        std::fs::create_dir_all(&icon_dir).ok();
        
        let mut icon_path = icon_dir.clone();
        icon_path.push("screamshot.png");
        
        // Generate the raw icon bytes and save them as a PNG!
        let (rgba, width, height) = generate_icon_raw();
        if let Some(rgba_img) = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(width, height, rgba) {
            let _ = rgba_img.save(&icon_path);
        }
        
        let mut app_dir = std::path::PathBuf::from(&home);
        app_dir.push(".local");
        app_dir.push("share");
        app_dir.push("applications");
        std::fs::create_dir_all(&app_dir).ok();
        
        let mut desktop_file = app_dir;
        desktop_file.push("screamshot.desktop");
        
        let content = format!(
            "[Desktop Entry]\n\
            Type=Application\n\
            Name=Screamshot\n\
            Exec={}\n\
            Icon={}\n\
            Comment=Region-Based Scrolling Capture\n\
            Terminal=false\n\
            Categories=Utility;\n",
            current_exe_str,
            icon_path.to_string_lossy()
        );
        
        let should_write = if let Ok(existing) = std::fs::read_to_string(&desktop_file) {
            existing != content
        } else {
            true
        };
        
        if should_write {
            let _ = std::fs::write(desktop_file, content);
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    ensure_desktop_entry();

    #[cfg(target_os = "linux")]
    if let Err(err) = gtk::init() {
        eprintln!("Failed to initialize GTK: {}", err);
    }
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_visible(false)
            .with_icon(std::sync::Arc::new(generate_egui_icon())),
        ..Default::default()
    };
    eframe::run_native(
        "Screamshot",
        options,
        Box::new(|_cc| Ok(Box::new(ScreamshotApp::new(_cc)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;

    #[test]
    #[cfg(target_os = "linux")]
    fn test_monitors() {
        use gtk::prelude::*;

        // Initialize GTK so GDK is active
        let _ = gtk::init();

        println!("--- XCAP Monitors ---");
        if let Ok(monitors) = xcap::Monitor::all() {
            for (i, m) in monitors.iter().enumerate() {
                println!(
                    "Monitor {}: x={:?}, y={:?}, width={:?}, height={:?}, is_primary={:?}",
                    i,
                    m.x(),
                    m.y(),
                    m.width(),
                    m.height(),
                    m.is_primary()
                );
            }
        }

        println!("--- GDK Pointer ---");
        if let Some(display) = gtk::gdk::Display::default() {
            if let Some(seat) = display.default_seat() {
                if let Some(pointer) = seat.pointer() {
                    let (_, x, y): (gtk::gdk::Screen, i32, i32) = pointer.position();
                    println!("Pointer position: x={}, y={}", x, y);
                    if let Ok(m) = xcap::Monitor::from_point(x, y) {
                        println!(
                            "XCAP Monitor at pointer: x={:?}, y={:?}, width={:?}, height={:?}",
                            m.x(),
                            m.y(),
                            m.width(),
                            m.height()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_desktop_bounds_logic() {
        let (x, y, w, h) = get_desktop_bounds();
        assert!(w > 0);
        assert!(h > 0);
        println!("Desktop bounds: x={}, y={}, w={}, h={}", x, y, w, h);
    }

    #[test]
    fn test_get_monitor_and_crop_coords_logic() {
        let (min_x, min_y, _, _) = get_desktop_bounds();
        if let Some((monitor, crop_x, crop_y)) = get_monitor_and_crop_coords(0.0, 0.0, 10.0, 10.0) {
            println!("Matched monitor: x={:?}, y={:?}", monitor.x(), monitor.y());
            assert_eq!(crop_x, (min_x + 10 - monitor.x().unwrap_or(0)) as u32);
            assert_eq!(crop_y, (min_y + 10 - monitor.y().unwrap_or(0)) as u32);
        }
    }

    #[test]
    fn test_find_overlap_perfect() {
        let mut img1 = RgbaImage::new(100, 100);
        let mut img2 = RgbaImage::new(100, 100);

        // Fill img1 with a gradient
        for (x, y, pixel) in img1.enumerate_pixels_mut() {
            *pixel = image::Rgba([x as u8, y as u8, 0, 255]);
        }

        // Fill img2 such that it overlaps with the bottom 40 pixels of img1
        // img1's bottom 40 pixels (y: 60..100) are copied to img2's top 40 pixels (y: 0..40)
        for x in 0..100 {
            for y in 0..40 {
                *img2.get_pixel_mut(x, y) = *img1.get_pixel(x, 60 + y);
            }
            // Fill the rest of img2 with something else
            for y in 40..100 {
                *img2.get_pixel_mut(x, y) = image::Rgba([x as u8, y as u8 + 100, 50, 255]);
            }
        }

        let overlap = find_overlap(&img1, &img2);
        assert_eq!(overlap, 40);
    }

    #[test]
    fn test_find_overlap_none() {
        let mut img1 = RgbaImage::new(100, 100);
        let mut img2 = RgbaImage::new(100, 100);

        // Set completely different colors
        for pixel in img1.pixels_mut() {
            *pixel = image::Rgba([255, 0, 0, 255]);
        }
        for pixel in img2.pixels_mut() {
            *pixel = image::Rgba([0, 255, 0, 255]);
        }

        let overlap = find_overlap(&img1, &img2);
        assert_eq!(overlap, 0);
    }

    #[test]
    fn test_stitch_frames() {
        let mut img1 = RgbaImage::new(100, 100);
        let mut img2 = RgbaImage::new(100, 100);

        for (x, y, pixel) in img1.enumerate_pixels_mut() {
            *pixel = image::Rgba([x as u8, y as u8, 0, 255]);
        }
        for x in 0..100 {
            for y in 0..40 {
                *img2.get_pixel_mut(x, y) = *img1.get_pixel(x, 60 + y);
            }
            for y in 40..100 {
                *img2.get_pixel_mut(x, y) = image::Rgba([x as u8, y as u8 + 100, 50, 255]);
            }
        }

        let stitched = stitch_frames(vec![img1, img2]);
        // Height should be 100 + (100 - 40) = 160
        assert_eq!(stitched.width(), 100);
        assert_eq!(stitched.height(), 160);
    }

    #[test]
    fn test_find_overlap_horizontal_perfect() {
        let mut img1 = RgbaImage::new(100, 100);
        let mut img2 = RgbaImage::new(100, 100);

        // Fill img1 with a gradient
        for (x, y, pixel) in img1.enumerate_pixels_mut() {
            *pixel = image::Rgba([x as u8, y as u8, 0, 255]);
        }

        // Fill img2 such that it overlaps with the right 40 pixels of img1
        // img1's right 40 pixels (x: 60..100) are copied to img2's left 40 pixels (x: 0..40)
        for y in 0..100 {
            for x in 0..40 {
                *img2.get_pixel_mut(x, y) = *img1.get_pixel(60 + x, y);
            }
            // Fill the rest of img2 with something else
            for x in 40..100 {
                *img2.get_pixel_mut(x, y) = image::Rgba([x as u8 + 100, y as u8, 50, 255]);
            }
        }

        let overlap = find_overlap_horizontal(&img1, &img2);
        assert_eq!(overlap, 40);
    }

    #[test]
    fn test_stitch_frames_horizontal() {
        let mut img1 = RgbaImage::new(100, 100);
        let mut img2 = RgbaImage::new(100, 100);

        for (x, y, pixel) in img1.enumerate_pixels_mut() {
            *pixel = image::Rgba([x as u8, y as u8, 0, 255]);
        }
        for y in 0..100 {
            for x in 0..40 {
                *img2.get_pixel_mut(x, y) = *img1.get_pixel(60 + x, y);
            }
            for x in 40..100 {
                *img2.get_pixel_mut(x, y) = image::Rgba([x as u8 + 100, y as u8, 50, 255]);
            }
        }

        let stitched = stitch_frames_horizontal(vec![img1, img2]);
        // Width should be 100 + (100 - 40) = 160
        assert_eq!(stitched.width(), 160);
        assert_eq!(stitched.height(), 100);
    }
}
