//! Xavier UI - Standalone egui desktop application
//!
//! Run with: cargo run --features egui-standalone --bin xavier-gui
//!
//! Or build: cargo build --features egui-standalone --release --bin xavier-gui

use eframe::egui;
use xavier::ui::KanbanState;

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
}

impl XavierApp {
    fn new() -> Self {
        Self {
            state: KanbanState::new(),
        }
    }
}

impl eframe::App for XavierApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update state and render
        self.state.render(ctx);

        // Request repaint for animations
        ctx.request_repaint();
    }
}
