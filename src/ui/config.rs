//! Configuration UI - Form for editing Xavier settings

use crate::settings::XavierSettings;
use egui::*;

pub struct ConfigView {
    /// Local copy of settings to allow cancelling
    settings: XavierSettings,
    /// Status message for the user
    status_message: String,
    /// Message color
    status_color: Color32,
}

impl ConfigView {
    pub fn new() -> Self {
        Self {
            settings: XavierSettings::current(),
            status_message: String::new(),
            status_color: Color32::GRAY,
        }
    }

    /// Render the configuration form
    pub fn render(&mut self, ui: &mut Ui) {
        // Apply Mint Green LED styling
        let mint_green = Color32::from_rgb(0, 255, 65);
        let dark_bg = Color32::from_rgb(10, 10, 10);

        ui.visuals_mut().override_text_color = Some(mint_green);
        ui.visuals_mut().widgets.noninteractive.bg_fill = dark_bg;
        ui.visuals_mut().widgets.inactive.bg_fill = dark_bg;

        ui.vertical_centered(|ui| {
            ui.heading(RichText::new("Xavier Configuration").color(mint_green).strong());
        });

        ui.add_space(20.0);

        egui::Grid::new("config_grid")
            .num_columns(2)
            .spacing([20.0, 10.0])
            .show(ui, |ui| {
                // Server Port
                ui.label("API Port:");
                ui.add(egui::DragValue::new(&mut self.settings.server.port).range(1024..=65535));
                ui.end_row();

                // Workspace ID
                ui.label("Workspace ID:");
                ui.text_edit_singleline(&mut self.settings.workspace.default_workspace_id);
                ui.end_row();

                // Auth Token
                ui.label("Auth Token:");
                let mut token_str = self.settings.server.token.clone().unwrap_or_default();
                if ui.text_edit_singleline(&mut token_str).changed() {
                    self.settings.server.token = if token_str.is_empty() {
                        None
                    } else {
                        Some(token_str)
                    };
                }
                ui.end_row();

                // Log Level
                ui.label("Log Level:");
                egui::ComboBox::from_id_source("log_level")
                    .selected_text(&self.settings.server.log_level)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.settings.server.log_level, "trace".into(), "Trace");
                        ui.selectable_value(&mut self.settings.server.log_level, "debug".into(), "Debug");
                        ui.selectable_value(&mut self.settings.server.log_level, "info".into(), "Info");
                        ui.selectable_value(&mut self.settings.server.log_level, "warn".into(), "Warn");
                        ui.selectable_value(&mut self.settings.server.log_level, "error".into(), "Error");
                    });
                ui.end_row();
            });

        ui.add_space(30.0);

        ui.horizontal(|ui| {
            if ui.button(RichText::new("Save Changes").color(Color32::BLACK).strong())
                .fill(mint_green)
                .clicked()
            {
                match self.settings.save() {
                    Ok(_) => {
                        self.status_message = "Settings saved successfully!".to_string();
                        self.status_color = mint_green;
                    }
                    Err(e) => {
                        self.status_message = format!("Error saving: {}", e);
                        self.status_color = Color32::LIGHT_RED;
                    }
                }
            }

            if ui.button("Reload").clicked() {
                self.settings = XavierSettings::current();
                self.status_message = "Settings reloaded.".to_string();
                self.status_color = Color32::GRAY;
            }
        });

        if !self.status_message.is_empty() {
            ui.add_space(10.0);
            ui.label(RichText::new(&self.status_message).color(self.status_color));
        }
    }
}
