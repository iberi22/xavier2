//! UI State - Manages UI state and connects to Xavier2 Task System

use crate::tasks::models::{Priority, Project, Task, TaskStatus};
use crate::ui::board::Column;
use egui::*;

/// Main UI state that bridges egui with Xavier2 tasks
pub struct KanbanState {
    /// Current tasks (cached)
    tasks: Vec<Task>,

    /// Current projects
    projects: Vec<Project>,

    /// Selected project filter
    selected_project: Option<String>,

    /// Columns configuration
    _columns: Vec<Column>,

    /// Search query
    search_query: String,

    /// Task modal
    selected_task: Option<Task>,

    /// Is modal open?
    modal_open: bool,

    /// UI state
    pub egui_state: EguiState,
}

/// Extra egui state
pub struct EguiState {
    pub side_panel_width: f32,
    pub is_dark_mode: bool,
}

impl KanbanState {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            projects: Self::default_projects(),
            selected_project: None,
            _columns: Column::default_columns(),
            search_query: String::new(),
            selected_task: None,
            modal_open: false,
            egui_state: EguiState {
                side_panel_width: 200.0,
                is_dark_mode: true,
            },
        }
    }

    /// Default projects for SouthLabs
    fn default_projects() -> Vec<Project> {
        vec![
            Project::new("Xavier2", "Sistema de memoria y cognitive"),
            Project::new("ZeroClaw", "Runtime Rust para agentes"),
            Project::new("Trading Bot", "Automatizacion de trading"),
            Project::new("ManteniApp", "SaaS mantenimiento industrial"),
            Project::new("Research", "Investigacion y experimentos"),
            Project::new("Ops", "Infraestructura y DevOps"),
        ]
    }

    /// Main render function
    pub fn render(&mut self, ctx: &Context) {
        // Load tasks (demo for now)
        if self.tasks.is_empty() {
            self.tasks = self.get_demo_tasks();
        }

        // Choose theme
        if self.egui_state.is_dark_mode {
            ctx.set_visuals(Visuals::dark());
        } else {
            ctx.set_visuals(Visuals::light());
        }

        // Main layout
        egui::SidePanel::left("sidebar")
            .default_width(self.egui_state.side_panel_width)
            .show(ctx, |ui| {
                self.render_sidebar(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_main(ui);
        });

        // Modal for task details
        if self.modal_open {
            self.render_modal(ctx);
        }
    }

    fn get_demo_tasks(&self) -> Vec<Task> {
        vec![
            {
                let mut t = Task::new("Integrar Planka con Xavier2", "Xavier2", "system");
                t.status = TaskStatus::InProgress;
                t.priority = Priority::High;
                t.labels = vec!["integration".to_string(), "backend".to_string()];
                t
            },
            {
                let mut t = Task::new("Crear UI nativa con egui", "Xavier2", "system");
                t.status = TaskStatus::Backlog;
                t.priority = Priority::Medium;
                t.labels = vec!["ui".to_string(), "frontend".to_string()];
                t
            },
            {
                let mut t = Task::new("Configurar CI/CD", "Ops", "system");
                t.status = TaskStatus::Done;
                t.priority = Priority::Low;
                t.labels = vec!["devops".to_string()];
                t
            },
        ]
    }

    fn render_sidebar(&mut self, ui: &mut Ui) {
        ui.heading("Xavier2");
        ui.separator();

        // Theme toggle
        ui.horizontal(|ui| {
            ui.label("Theme:");
            if ui
                .button(if self.egui_state.is_dark_mode {
                    "🌙"
                } else {
                    "☀️"
                })
                .clicked()
            {
                self.egui_state.is_dark_mode = !self.egui_state.is_dark_mode;
            }
        });

        ui.separator();

        // Search
        ui.label("Search:");
        ui.text_edit_singleline(&mut self.search_query);

        ui.separator();

        // Projects
        ui.label("Projects:");

        egui::ScrollArea::vertical().show(ui, |ui| {
            // All projects option
            let all_selected = self.selected_project.is_none();
            if ui
                .selectable_label(all_selected, "📋 All Projects")
                .clicked()
            {
                self.selected_project = None;
            }

            // Each project
            for project in &self.projects {
                let is_selected = self
                    .selected_project
                    .as_ref()
                    .map(|s| s == &project.name)
                    .unwrap_or(false);
                let icon = Self::get_project_icon(&project.name);
                if ui
                    .selectable_label(is_selected, format!("{} {}", icon, project.name))
                    .clicked()
                {
                    self.selected_project = Some(project.name.clone());
                }
            }
        });

        // Add project button
        ui.separator();
        if ui.button("+ New Project").clicked() {
            // TODO: Open new project dialog
        }
    }

    fn render_main(&mut self, ui: &mut Ui) {
        // Filter tasks by selected project and search
        let filtered_tasks: Vec<_> = self
            .tasks
            .iter()
            .filter(|t| {
                // Filter by project
                if let Some(ref proj) = self.selected_project {
                    if &t.project != proj {
                        return false;
                    }
                }
                // Filter by search
                if !self.search_query.is_empty() {
                    let q = self.search_query.to_lowercase();
                    if !t.title.to_lowercase().contains(&q)
                        && !t.description.to_lowercase().contains(&q)
                    {
                        return false;
                    }
                }
                true
            })
            .collect();

        // Board header
        ui.horizontal(|ui| {
            if let Some(ref proj) = self.selected_project {
                ui.heading(proj);
            } else {
                ui.heading("All Projects");
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("{} tasks", filtered_tasks.len()));
            });
        });

        ui.separator();

        // Kanban board
        super::board::BoardView::render(
            ui,
            &filtered_tasks,
            &self.projects,
            self.selected_project.as_deref(),
        );
    }

    fn render_modal(&mut self, ctx: &Context) {
        egui::Window::new("Task Details")
            .collapsible(false)
            .resizable(true)
            .open(&mut self.modal_open)
            .show(ctx, |ui| {
                if let Some(ref task) = self.selected_task {
                    ui.label(&task.title);
                    ui.separator();
                    ui.label(&task.description);
                }
            });

        if !self.modal_open {
            self.selected_task = None;
        }
    }

    fn get_project_icon(name: &str) -> &'static str {
        match name.to_lowercase().as_str() {
            "xavier2" => "🧠",
            "zeroclaw" => "⚡",
            "trading bot" => "📈",
            "manteniapp" => "🔧",
            "research" => "🔬",
            "ops" => "🚀",
            _ => "📁",
        }
    }
}

impl Default for KanbanState {
    fn default() -> Self {
        Self::new()
    }
}
