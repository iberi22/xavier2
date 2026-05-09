//! Xavier UI - Standalone egui desktop application
//!
//! Run with: cargo run --features egui-standalone --bin xavier-gui
//!
//! Or build: cargo build --features egui-standalone --release --bin xavier-gui

use eframe::egui;
use xavier::ui::{ConfigView, KanbanState};

fn main() -> eframe::Result<()> {
    // Parse arguments
    let args: Vec<String> = std::env::args().collect();
    let is_config_mode = args.iter().any(|arg| arg == "--config");

    // Configure native options for desktop
    let mut native_options = eframe::NativeOptions::default();

    if is_config_mode {
        native_options.viewport.title = Some("Xavier Configuration".to_string());
        native_options.viewport.inner_size = Some(egui::vec2(400.0, 450.0));
        native_options.viewport.resizable = Some(false);
        native_options.viewport.decorated = Some(false);
        native_options.viewport.always_on_top = Some(true);
    } else {
        native_options.viewport.title = Some("Xavier - Kanban Board".to_string());
        native_options.viewport.inner_size = Some(egui::vec2(1200.0, 800.0));
        native_options.viewport.min_inner_size = Some(egui::vec2(800.0, 600.0));
    }

    // Run the application
    eframe::run_native(
        "Xavier",
        native_options,
        Box::new(move |_cc| Ok(Box::new(XavierApp::new(is_config_mode)))),
    )
}

/// Main application struct
struct XavierApp {
    state: KanbanState,
    config_view: ConfigView,
    is_config_mode: bool,
}

impl XavierApp {
    fn new(is_config_mode: bool) -> Self {
        Self {
            state: KanbanState::new(),
            config_view: ConfigView::new(),
            is_config_mode,
        }
    }
}

impl eframe::App for XavierApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.is_config_mode {
            egui::CentralPanel::default()
                .frame(egui::Frame::window(&ctx.style()).fill(egui::Color32::from_rgb(10, 10, 10)))
                .show(ctx, |ui| {
                // Drag the window by clicking anywhere in the background
                if ui.interact(ui.max_rect(), ui.id(), egui::Sense::drag()).dragged() {
                    frame.drag_window();
                }

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("X").clicked() {
                                frame.close();
                            }
                        });
                    });

                    self.config_view.render(ui);
                });
            });
        } else {
            // Update state and render main Kanban board
            self.state.render(ctx);
        }

        // Request repaint for animations
        ctx.request_repaint();
    }
}
