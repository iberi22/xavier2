use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder, Icon, TrayIconEvent,
};
use image::{RgbaImage, Rgba};
use crate::settings::XavierSettings;

pub struct SystemTray {
    tray_icon: Option<TrayIcon>,
    _menu: Menu,
    item_status: MenuItem,
    item_start_stop: MenuItem,
    item_open_config: MenuItem,
    item_dashboard: MenuItem,
    item_exit: MenuItem,
    is_active: bool,
}

impl SystemTray {
    pub fn new() -> Self {
        let menu = Menu::new();
        let item_status = MenuItem::new("Status: Unknown", false, None);
        let item_start_stop = MenuItem::new("Start Server", true, None);
        let item_open_config = MenuItem::new("Open Config", true, None);
        let item_dashboard = MenuItem::new("Dashboard", true, None);
        let item_exit = MenuItem::new("Exit", true, None);

        menu.append_items(&[
            &item_status,
            &PredefinedMenuItem::separator(),
            &item_start_stop,
            &item_open_config,
            &item_dashboard,
            &PredefinedMenuItem::separator(),
            &item_exit,
        ]).unwrap();

        let initial_icon = Self::generate_triangle_icon(0.5, false);
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu.clone()))
            .with_tooltip("Xavier")
            .with_icon(initial_icon)
            .build()
            .ok();

        Self {
            tray_icon,
            _menu: menu,
            item_status,
            item_start_stop,
            item_open_config,
            item_dashboard,
            item_exit,
            is_active: false,
        }
    }

    /// Generates a 3-dot triangle icon programmatically.
    /// glow: intensity from 0.0 to 1.0
    /// active: if true, uses mint green; if false, uses dim gray
    pub fn generate_triangle_icon(glow: f32, active: bool) -> Icon {
        let size = 32;
        let mut img = RgbaImage::new(size, size);

        let color = if active {
            // Mint green (152, 255, 152) with glow
            let g = (200.0 + 55.0 * glow) as u8;
            Rgba([152, g, 152, 255])
        } else {
            // Dim gray (105, 105, 105)
            Rgba([80, 80, 80, 255])
        };

        // Triangle coordinates (3 dots)
        let dots = [
            (size / 2, size / 4),          // Top
            (size / 4, 3 * size / 4),      // Bottom Left
            (3 * size / 4, 3 * size / 4),  // Bottom Right
        ];

        let radius = 4;
        for (cx, cy) in dots {
            for x in (cx - radius)..(cx + radius) {
                for y in (cy - radius)..(cy + radius) {
                    let dx = x as i32 - cx as i32;
                    let dy = y as i32 - cy as i32;
                    if dx * dx + dy * dy <= (radius * radius) as i32 {
                        if x < size && y < size {
                            img.put_pixel(x, y, color);
                        }
                    }
                }
            }
        }

        let rgba = img.into_raw();
        Icon::from_rgba(rgba, size, size).expect("Failed to create icon")
    }

    pub fn set_icon(&mut self, icon: Icon) {
        if let Some(ref mut tray) = self.tray_icon {
            let _ = tray.set_icon(Some(icon));
        }
    }

    pub fn update_status_text(&mut self, active: bool) {
        if self.is_active == active {
            return;
        }
        self.is_active = active;

        let status_text = if active { "Status: Active" } else { "Status: Inactive" };
        self.item_status.set_text(status_text);

        let start_stop_text = if active { "Stop Server" } else { "Start Server" };
        self.item_start_stop.set_text(start_stop_text);
    }

    pub fn handle_events(&self) -> TrayAction {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.item_exit.id() {
                return TrayAction::Exit;
            } else if event.id == self.item_open_config.id() {
                return TrayAction::OpenConfig;
            } else if event.id == self.item_dashboard.id() {
                let settings = XavierSettings::current();
                let url = settings.client_base_url();
                let _ = webbrowser::open(&url);
                return TrayAction::OpenDashboard;
            } else if event.id == self.item_start_stop.id() {
                return TrayAction::ToggleServer;
            }
        }

        if let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if event == TrayIconEvent::Click {
                return TrayAction::ToggleWindow;
            }
        }

        TrayAction::None
    }
}

pub enum TrayAction {
    None,
    Exit,
    OpenConfig,
    OpenDashboard,
    ToggleServer,
    ToggleWindow,
}
