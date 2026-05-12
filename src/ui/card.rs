//! Card View - Individual task card rendering

use crate::tasks::models::{Priority, Task};
#[cfg(feature = "egui")]
use egui::*;

/// Render a single task card
pub struct CardView;

#[cfg(feature = "egui")]
impl CardView {
    pub fn render(ui: &mut Ui, task: &Task) {
        // Card background with hover effect
        let card_response = egui::Frame::group(ui.style())
            .fill(ui.style().visuals.panel_fill)
            .stroke(ui.style().visuals.window_stroke())
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Title
                    ui.label(egui::RichText::new(&task.title).small());

                    // Description preview (truncated)
                    if !task.description.is_empty() {
                        let preview = if task.description.len() > 80 {
                            format!("{}...", &task.description[..80])
                        } else {
                            task.description.clone()
                        };
                        ui.label(egui::RichText::new(preview).small().weak());
                    }

                    // Labels/Tags
                    if !task.labels.is_empty() {
                        ui.horizontal(|ui| {
                            for label in task.labels.iter().take(3) {
                                Self::render_label(ui, label);
                            }
                            if task.labels.len() > 3 {
                                ui.label(
                                    egui::RichText::new(format!("+{}", task.labels.len() - 3))
                                        .small(),
                                );
                            }
                        });
                    }

                    // Footer: priority + assignee
                    ui.horizontal(|ui| {
                        // Priority indicator
                        Self::render_priority(ui, task.priority);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Assignee avatar or initials
                            if let Some(assignee) = &task.assignee {
                                let initials = Self::get_initials(assignee);
                                ui.label(
                                    egui::RichText::new(initials)
                                        .small()
                                        .color(Color32::from_rgb(156, 163, 175)),
                                );
                            }
                        });
                    });
                });
            });

        // Handle click to open detail view
        if card_response
            .response
            .interact(egui::Sense::click())
            .clicked()
        {
            // Open task detail modal (TUI — to be implemented)
        }
    }

    fn render_label(ui: &mut Ui, label: &str) {
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(99, 102, 241)) // Indigo
            .corner_radius(2.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new(label).small().color(Color32::WHITE));
            });
    }

    fn render_priority(ui: &mut Ui, priority: Priority) {
        let (color, text) = match priority {
            Priority::Low => (Color32::from_rgb(156, 163, 175), "L"),
            Priority::Medium => (Color32::from_rgb(251, 191, 36), "M"),
            Priority::High => (Color32::from_rgb(249, 115, 22), "H"),
            Priority::Urgent => (Color32::from_rgb(239, 68, 68), "U"),
        };

        egui::Frame::group(ui.style())
            .fill(color)
            .corner_radius(2.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(text)
                        .small()
                        .color(Color32::WHITE)
                        .strong(),
                );
            });
    }

    fn get_initials(name: &str) -> String {
        let parts: Vec<&str> = name.split_whitespace().collect();
        if parts.len() >= 2 {
            format!(
                "{}{}",
                parts[0].chars().next().unwrap_or('?'),
                parts[1].chars().next().unwrap_or('?')
            )
            .to_uppercase()
        } else {
            name.chars().take(2).collect::<String>().to_uppercase()
        }
    }
}

/// Modal for viewing/editing task details
pub struct TaskModal {
    pub task: Option<Task>,
    pub is_open: bool,
}

impl TaskModal {
    pub fn new() -> Self {
        Self {
            task: None,
            is_open: false,
        }
    }

    pub fn open(&mut self, task: Task) {
        self.task = Some(task);
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.task = None;
    }

    #[cfg(feature = "egui")]
    pub fn render(&mut self, ui: &mut Ui) {
        if !self.is_open {
            return;
        }

        let mut is_open = self.is_open;
        egui::Window::new("Task Details")
            .collapsible(false)
            .resizable(true)
            .open(&mut is_open)
            .show(ui.ctx(), |ui| {
                if let Some(ref task) = self.task {
                    self.render_task_details(ui, task);
                }
            });

        self.is_open = is_open;
        if !self.is_open {
            self.close();
        }
    }

    #[cfg(feature = "egui")]
    fn render_task_details(&self, ui: &mut Ui, task: &Task) {
        egui::Grid::new("task_details")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                // Title
                ui.label("Title:");
                ui.label(&task.title);
                ui.end_row();

                // Description
                ui.label("Description:");
                ui.end_row();
                ui.horizontal_wrapped(|ui| {
                    ui.label(&task.description);
                });
                ui.end_row();

                // Status
                ui.label("Status:");
                ui.label(format!("{:?}", task.status));
                ui.end_row();

                // Priority
                ui.label("Priority:");
                ui.label(format!("{:?}", task.priority));
                ui.end_row();

                // Assignee
                ui.label("Assignee:");
                ui.label(task.assignee.as_deref().unwrap_or("Unassigned"));
                ui.end_row();

                // Created
                ui.label("Created:");
                ui.label(task.created_at.format("%Y-%m-%d %H:%M").to_string());
                ui.end_row();

                // Labels
                if !task.labels.is_empty() {
                    ui.label("Labels:");
                    ui.horizontal(|ui| {
                        for label in &task.labels {
                            Self::render_label(ui, label);
                        }
                    });
                }
            });
    }

    #[cfg(feature = "egui")]
    fn render_label(ui: &mut Ui, label: &str) {
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(99, 102, 241))
            .corner_radius(2.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new(label).small().color(Color32::WHITE));
            });
    }
}

impl Default for TaskModal {
    fn default() -> Self {
        Self::new()
    }
}
