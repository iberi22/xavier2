//! Xavier2 Web - WASM entry point for web-based UI

#[cfg(target_arch = "wasm32")]
use xavier2::ui::{KanbanState};
#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::prelude::*;

/// Start the egui web application
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    // Set up panic hook for better error messages
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    // Log startup
    web_sys::console::log_1(&"Xavier2 UI starting...".into());

    // Run the application
    eframe::WebRunner::new()
        .start(
            "canvas", // canvas element ID
            eframe::WebOptions::default(),
            Box::new(|_cc| Ok(Box::new(Xavier2WebApp::new()))),
        )
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    web_sys::console::log_1(&"Xavier2 UI initialized!".into());

    Ok(())
}

/// Main web application struct
#[cfg(target_arch = "wasm32")]
struct Xavier2WebApp {
    state: KanbanState,
}

#[cfg(target_arch = "wasm32")]
impl Xavier2WebApp {
    fn new() -> Self {
        Self {
            state: KanbanState::new(),
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl eframe::App for Xavier2WebApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.state.render(ctx);
        ctx.request_repaint();
    }
}
