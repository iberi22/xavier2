//! Kanban Board View - Main board rendering with egui

use crate::tasks::models::{Project, Task, TaskStatus};
#[cfg(feature = "egui")]
use egui::*;

/// Render a kanban board with columns and cards
pub struct BoardView;

#[cfg(feature = "egui")]
impl BoardView {
    /// Render the full kanban board
    pub fn render(
        ui: &mut Ui,
        tasks: &[&Task],
        projects: &[Project],
        selected_project: Option<&str>,
    ) {
        // Header with project selector
        Self::render_header(ui, projects, selected_project);

        ui.separator();

        // Main board area with horizontal scrolling
        egui::ScrollArea::horizontal()
            .stick_to_right(true)
            .show(ui, |ui| {
                // Get tasks for each column
                let backlog_tasks: Vec<&Task> = tasks
                    .iter()
                    .filter(|t| t.status == TaskStatus::Backlog)
                    .copied()
                    .collect();
                let in_progress_tasks: Vec<&Task> = tasks
                    .iter()
                    .filter(|t| t.status == TaskStatus::InProgress)
                    .copied()
                    .collect();
                let done_tasks: Vec<&Task> = tasks
                    .iter()
                    .filter(|t| t.status == TaskStatus::Done)
                    .copied()
                    .collect();

                // Render columns
                egui::Grid::new("kanban_columns")
                    .num_columns(3)
                    .spacing([16.0, 0.0])
                    .show(ui, |ui| {
                        Self::render_column(ui, "Backlog", &backlog_tasks, TaskStatus::Backlog);
                        Self::render_column(
                            ui,
                            "In Progress",
                            &in_progress_tasks,
                            TaskStatus::InProgress,
                        );
                        Self::render_column(ui, "Done", &done_tasks, TaskStatus::Done);
                    });
            });
    }

    fn render_header(ui: &mut Ui, _projects: &[Project], _selected: Option<&str>) {
        egui::Grid::new("header")
            .num_columns(2)
            .spacing([8.0, 0.0])
            .show(ui, |ui| {
                ui.label("Project:");
                // Project selector - simplified for now
                ui.label(_selected.unwrap_or("All"));

                ui.end_row();

                // Add task button
                if ui.button("+ New Task").clicked() {
                    // TODO: Open new task dialog
                }
            });
    }

    fn render_column(ui: &mut Ui, title: &str, tasks: &[&Task], _status: TaskStatus) {
        // Column header
        ui.vertical(|ui| {
            egui::Frame::group(ui.style())
                .fill(ui.style().visuals.code_bg_color)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(title).heading().strong());
                        ui.label(egui::RichText::new(format!("({})", tasks.len())).small());
                    });
                });

            // Cards in this column
            for task in tasks {
                super::card::CardView::render(ui, task);
            }

            // Drop zone / Add card
            ui.horizontal(|ui| {
                if ui.button("+ Add card").clicked() {
                    // TODO: Add card to this column
                }
            });
        });
    }
}

/// Column configuration
#[derive(Debug, Clone)]
pub struct Column {
    pub id: String,
    pub name: String,
    pub status: TaskStatus,
    #[cfg(feature = "egui")]
    pub color: Color32,
}

impl Column {
    pub fn default_columns() -> Vec<Self> {
        vec![
            Column {
                id: "backlog".to_string(),
                name: "Backlog".to_string(),
                status: TaskStatus::Backlog,
                #[cfg(feature = "egui")]
                color: Color32::from_rgb(100, 116, 139), // Gray
            },
            Column {
                id: "in_progress".to_string(),
                name: "In Progress".to_string(),
                status: TaskStatus::InProgress,
                #[cfg(feature = "egui")]
                color: Color32::from_rgb(59, 130, 246), // Blue
            },
            Column {
                id: "done".to_string(),
                name: "Done".to_string(),
                status: TaskStatus::Done,
                #[cfg(feature = "egui")]
                color: Color32::from_rgb(34, 197, 94), // Green
            },
        ]
    }
}
