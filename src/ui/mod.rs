//! Kanban UI - Native egui-based kanban board for Xavier
//!
//! This provides a native kanban UI that can run:
//! - As a desktop app (via eframe)
//! - In the browser (via WASM + web_sys)
//! - Embedded in the Xavier HTTP server
//!
//! ## Usage
//!
//! ```rust,ignore
//! use xavier::ui::KanbanState;
//!
//! let app = KanbanState::new();
//! ```

// UI modules - only compile with egui feature
#[cfg(feature = "egui")]
pub mod board;
#[cfg(feature = "egui")]
pub mod card;
#[cfg(feature = "egui")]
pub mod state;
#[cfg(feature = "egui")]
pub mod tray;

#[cfg(feature = "cli-interactive")]
pub mod dashboard;
#[cfg(feature = "cli-interactive")]
pub mod log_stream;
#[cfg(feature = "cli-interactive")]
pub mod memory_view;

#[cfg(feature = "egui")]
pub use board::BoardView;
#[cfg(feature = "egui")]
pub use card::CardView;
#[cfg(feature = "egui")]
pub use state::{EguiState, KanbanState};
#[cfg(feature = "egui")]
pub use tray::{SystemTray, TrayAction};
