//! Xavier2 UI - Standalone egui desktop application
//!
//! Run with: cargo run --features egui-standalone --bin xavier2-gui
//!
//! Or build: cargo build --features egui-standalone --release --bin xavier2-gui

use eframe::egui;
use xavier2::ui::KanbanState;

fn main() -> eframe::Result<()> {
    // Configure native options for desktop
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport.title = Some("Xavier2 - Kanban Board".to_string());
    native_options.viewport.inner_size = Some(egui::vec2(1200.0, 800.0));
    native_options.viewport.min_inner_size = Some(egui::vec2(800.0, 600.0));

    // Run the application
    eframe::run_native(
        "Xavier2",
        native_options,
        Box::new(|_cc| Ok(Box::new(Xavier2App::new()))),
    )
}

/// Main application struct
struct Xavier2App {
    state: KanbanState,
}

impl Xavier2App {
    fn new() -> Self {
        Self {
            state: KanbanState::new(),
        }
    }
}

impl eframe::App for Xavier2App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update state and render
        self.state.render(ctx);

        // Request repaint for animations
        ctx.request_repaint();
    }
}
