//! Xavier UI - Standalone egui desktop application
//!
//! Run with: cargo run --features egui-standalone --bin xavier-gui
//!
//! Or build: cargo build --features egui-standalone --release --bin xavier-gui

use eframe::egui;
use xavier::ui::{KanbanState, SystemTray, TrayAction};
use std::time::{Duration, Instant};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

fn main() -> eframe::Result<()> {
    // Configure native options for desktop
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport.title = Some("Xavier - Kanban Board".to_string());
    native_options.viewport.inner_size = Some(egui::vec2(1200.0, 800.0));
    native_options.viewport.min_inner_size = Some(egui::vec2(800.0, 600.0));

    // Run the application
    eframe::run_native(
        "Xavier",
        native_options,
        Box::new(|_cc| Ok(Box::new(XavierApp::new()))),
    )
}

/// Main application struct
struct XavierApp {
    state: KanbanState,
    tray: SystemTray,
    last_health_check: Instant,
    last_animation_frame: Instant,
    is_visible: bool,
    server_active: Arc<AtomicBool>,
}

impl XavierApp {
    fn new() -> Self {
        Self {
            state: KanbanState::new(),
            tray: SystemTray::new(),
            last_health_check: Instant::now() - Duration::from_secs(60), // Force check on start
            last_animation_frame: Instant::now(),
            is_visible: true,
            server_active: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl eframe::App for XavierApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle Tray Events
        match self.tray.handle_events() {
            TrayAction::Exit => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            TrayAction::ToggleWindow | TrayAction::OpenConfig => {
                self.is_visible = !self.is_visible;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(self.is_visible));
            }
            TrayAction::OpenDashboard => {
                // Placeholder
            }
            TrayAction::ToggleServer => {
                // Placeholder
            }
            TrayAction::None => {}
        }

        // Update Tray Icon based on background health check and animation
        let is_active = self.server_active.load(Ordering::SeqCst);
        self.tray.update_status_text(is_active);

        if is_active {
            // Pulse animation every 100ms
            if self.last_animation_frame.elapsed() > Duration::from_millis(100) {
                self.last_animation_frame = Instant::now();
                let time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f32();
                let glow = (time.sin() * 0.5 + 0.5); // 0.0 to 1.0
                let icon = SystemTray::generate_triangle_icon(glow, true);
                self.tray.set_icon(icon);
            }
        } else {
            // Static inactive icon
            let icon = SystemTray::generate_triangle_icon(0.0, false);
            self.tray.set_icon(icon);
        }

        // Periodic health check
        if self.last_health_check.elapsed() > Duration::from_secs(10) {
            self.last_health_check = Instant::now();
            let settings = xavier::settings::XavierSettings::current();
            let url = format!("{}/health", settings.client_base_url());
            let server_active = self.server_active.clone();

            std::thread::spawn(move || {
                let client = reqwest::blocking::Client::new();
                let res = client.get(url).timeout(Duration::from_secs(2)).send();
                let is_ok = res.is_ok() && res.unwrap().status().is_success();
                server_active.store(is_ok, Ordering::SeqCst);
            });
        }

        if self.is_visible {
            // Update state and render
            self.state.render(ctx);
        }

        // Request repaint for animations and tray event polling
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
