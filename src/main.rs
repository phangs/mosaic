use eframe::egui;
use image::{ImageBuffer, Rgba, RgbaImage};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder, Icon,
};
use std::sync::{Arc, Mutex};
use enigo::{Enigo, Keyboard, Key, Settings, Direction};

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

fn update_autostart(enabled: bool) {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let mut autostart_dir = std::path::PathBuf::from(home);
    autostart_dir.push(".config");
    autostart_dir.push("autostart");
    std::fs::create_dir_all(&autostart_dir).ok();
    
    let mut desktop_file = autostart_dir;
    desktop_file.push("screamshot.desktop");

    if enabled {
        if let Ok(current_exe) = std::env::current_exe() {
            let current_exe_str = current_exe.to_string_lossy();
            let content = format!(
                "[Desktop Entry]\n\
                Type=Application\n\
                Name=Screamshot\n\
                Exec={}\n\
                Icon=camera-photo\n\
                Comment=Region-Based Scrolling Capture\n\
                Terminal=false\n",
                current_exe_str
            );
            let _ = std::fs::write(desktop_file, content);
        }
    } else {
        let _ = std::fs::remove_file(desktop_file);
    }
}

fn generate_icon() -> Icon {
    let width = 32;
    let height = 32;
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(width, height);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let r = (x as f32 / width as f32 * 255.0) as u8;
        let b = (y as f32 / height as f32 * 255.0) as u8;
        *pixel = Rgba([r, 0, b, 255]);
    }
    Icon::from_rgba(img.into_raw(), width, height).expect("Failed to create icon")
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

    let summary = if prefix == "scroll" {
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
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            } else if event.id == self.capture_scrolling_i.id() {
                self.state = AppState::SelectingScrollRegion;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
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
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Scrolling Capture");
                
                let num_frames = self.scroll_frames.lock().unwrap().len();
                ui.label(format!("Captured frames: {}", num_frames));
                ui.separator();
                
                ui.label("Trigger auto-scroll & capture:");
                
                ui.vertical(|ui| {
                    if ui.button("Scroll Page & Capture").clicked() {
                        if let Some(rect) = self.scroll_rect {
                            let frames_arc = Arc::clone(&self.scroll_frames);
                            let ctx_clone = ctx.clone();
                            
                            // Hide the control panel window FIRST so it doesn't block the screen
                            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                            
                            std::thread::spawn(move || {
                                std::thread::sleep(std::time::Duration::from_millis(200));
                                
                                // Simulate PageDown key press
                                if let Ok(mut enigo) = Enigo::new(&Settings::default()) {
                                    let _ = enigo.key(Key::PageDown, Direction::Click);
                                }
                                
                                // Wait for smooth scrolling to settle
                                std::thread::sleep(std::time::Duration::from_millis(500));
                                
                                // Capture the new frame
                                if let Ok(monitors) = xcap::Monitor::all() {
                                    if let Some(monitor) = monitors.first() {
                                        if let Ok(mut img) = monitor.capture_image() {
                                            let min_x = rect.min.x as u32;
                                            let min_y = rect.min.y as u32;
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
                                
                                // Re-show our window
                                ctx_clone.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                            });
                        }
                    }
                    
                    if ui.button("Scroll Down (Line) & Capture").clicked() {
                        if let Some(rect) = self.scroll_rect {
                            let frames_arc = Arc::clone(&self.scroll_frames);
                            let ctx_clone = ctx.clone();
                            
                            // Hide the control panel window FIRST
                            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                            
                            std::thread::spawn(move || {
                                std::thread::sleep(std::time::Duration::from_millis(200));
                                
                                // Simulate DownArrow key press multiple times
                                if let Ok(mut enigo) = Enigo::new(&Settings::default()) {
                                    for _ in 0..5 {
                                        let _ = enigo.key(Key::DownArrow, Direction::Click);
                                        std::thread::sleep(std::time::Duration::from_millis(30));
                                    }
                                }
                                
                                // Wait for smooth scrolling to settle
                                std::thread::sleep(std::time::Duration::from_millis(400));
                                
                                // Capture the new frame
                                if let Ok(monitors) = xcap::Monitor::all() {
                                    if let Some(monitor) = monitors.first() {
                                        if let Ok(mut img) = monitor.capture_image() {
                                            let min_x = rect.min.x as u32;
                                            let min_y = rect.min.y as u32;
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
                                
                                // Re-show our window
                                ctx_clone.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                            });
                        }
                    }
                });
                
                ui.separator();
                
                ui.horizontal(|ui| {
                    if ui.button("Finish & Stitch").clicked() {
                        let frames = {
                            let mut frames_guard = self.scroll_frames.lock().unwrap();
                            std::mem::take(&mut *frames_guard)
                        };
                        
                        let cfg = self.config.clone();
                        if !frames.is_empty() {
                            std::thread::spawn(move || {
                                println!("Stitching {} frames...", frames.len());
                                let stitched = stitch_frames(frames);
                                save_and_clipboard(stitched, "scroll", &cfg);
                            });
                        }
                        
                        self.state = AppState::Hidden;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    }
                    
                    if ui.button("Cancel").clicked() {
                        self.scroll_frames.lock().unwrap().clear();
                        self.state = AppState::Hidden;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    }
                });
            });
            
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
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
                            let min_x = start.x.min(end.x) as u32;
                            let min_y = start.y.min(end.y) as u32;
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
                                    std::thread::spawn(move || {
                                        std::thread::sleep(std::time::Duration::from_millis(200));
                                        if let Ok(monitors) = xcap::Monitor::all() {
                                            if let Some(monitor) = monitors.first() {
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
                                    
                                    // Switch state and resize window FIRST to ensure overlay disappears
                                    self.state = AppState::CapturingScroll;
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(350.0, 200.0)));
                                    
                                    let center_x = screen_rect.width() / 2.0;
                                    let pos_x = if rect.center().x > center_x {
                                        50.0
                                    } else {
                                        screen_rect.width() - 400.0
                                    };
                                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(pos_x, 50.0)));
                                    
                                    // Capture first frame in background
                                    std::thread::spawn(move || {
                                        std::thread::sleep(std::time::Duration::from_millis(400));
                                        if let Ok(monitors) = xcap::Monitor::all() {
                                            if let Some(monitor) = monitors.first() {
                                                if let Ok(mut img) = monitor.capture_image() {
                                                    if min_x + width <= img.width() && min_y + height <= img.height() {
                                                        let cropped = image::imageops::crop(&mut img, min_x, min_y, width, height).to_image();
                                                        frames_arc.lock().unwrap().push(cropped);
                                                    }
                                                }
                                            }
                                        }
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

fn ensure_desktop_entry() {
    if let Ok(current_exe) = std::env::current_exe() {
        let current_exe_str = current_exe.to_string_lossy();
        
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
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
            Icon=camera-photo\n\
            Comment=Region-Based Scrolling Capture\n\
            Terminal=false\n\
            Categories=Utility;\n",
            current_exe_str
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

    if let Err(err) = gtk::init() {
        eprintln!("Failed to initialize GTK: {}", err);
    }
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_visible(false),
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
}
