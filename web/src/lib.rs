//! Xavier2 Web - Standalone web UI (WASM)

#![allow(dead_code)]

use eframe::egui;
use egui::*;
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

static ID_COUNTER: AtomicU32 = AtomicU32::new(1);

fn generate_id() -> String {
    let id = ID_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("id-{}", id)
}

// ============================================================================
// Task Models
// ============================================================================

#[derive(Debug, Clone, Copy, Default, PartialEq)]
enum Priority {
    Low,
    #[default]
    Medium,
    High,
    Urgent,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
enum TaskStatus {
    #[default]
    Backlog,
    InProgress,
    Done,
}

#[derive(Debug, Clone)]
struct Task {
    id: String,
    title: String,
    project: String,
    status: TaskStatus,
    priority: Priority,
    labels: Vec<String>,
}

impl Task {
    fn new(title: &str, project: &str) -> Self {
        Self {
            id: generate_id(),
            title: title.to_string(),
            project: project.to_string(),
            status: TaskStatus::Backlog,
            priority: Priority::Medium,
            labels: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct Project {
    id: String,
    name: String,
}

impl Project {
    fn new(name: &str) -> Self {
        Self {
            id: generate_id(),
            name: name.to_string(),
        }
    }
}

// ============================================================================
// Helper functions
// ============================================================================

fn project_icon(name: &str) -> &'static str {
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

fn render_column(ui: &mut Ui, title: &str, tasks: &[&Task], status: TaskStatus) {
    let column_tasks: Vec<_> = tasks.iter().filter(|t| t.status == status).collect();

    ui.vertical(|ui| {
        egui::Frame::group(ui.style())
            .fill(ui.style().visuals.code_bg_color)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(title).heading().strong());
                    ui.label(RichText::new(format!("({})", column_tasks.len())).small());
                });
            });

        for task in &column_tasks {
            render_card(ui, task);
        }
    });
}

fn render_card(ui: &mut Ui, task: &Task) {
    egui::Frame::group(ui.style())
        .fill(ui.style().visuals.panel_fill)
        .stroke(ui.style().visuals.window_stroke())
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(&task.title).small());

                if !task.labels.is_empty() {
                    ui.horizontal(|ui| {
                        for label in task.labels.iter().take(3) {
                            render_label(ui, label);
                        }
                    });
                }

                ui.horizontal(|ui| {
                    render_priority(ui, task.priority);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(&task.project)
                                .small()
                                .color(Color32::from_rgb(156, 163, 175)),
                        );
                    });
                });
            });
        });
}

fn render_label(ui: &mut Ui, label: &str) {
    egui::Frame::group(ui.style())
        .fill(Color32::from_rgb(99, 102, 241))
        .corner_radius(2.0)
        .show(ui, |ui| {
            ui.label(RichText::new(label).small().color(Color32::WHITE));
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
            ui.label(RichText::new(text).small().color(Color32::WHITE).strong());
        });
}

// ============================================================================
// Main App
// ============================================================================

struct Xavier2WebApp {
    tasks: Vec<Task>,
    projects: Vec<Project>,
    selected_project: Option<String>,
    search_query: String,
    is_dark_mode: bool,
}

impl Default for Xavier2WebApp {
    fn default() -> Self {
        Self {
            tasks: Self::demo_tasks(),
            projects: Self::default_projects(),
            selected_project: None,
            search_query: String::new(),
            is_dark_mode: true,
        }
    }
}

impl Xavier2WebApp {
    fn default_projects() -> Vec<Project> {
        vec![
            Project::new("Xavier2"),
            Project::new("ZeroClaw"),
            Project::new("Trading Bot"),
            Project::new("ManteniApp"),
            Project::new("Research"),
            Project::new("Ops"),
        ]
    }

    fn demo_tasks() -> Vec<Task> {
        vec![
            {
                let mut t = Task::new("Integrar Planka con Xavier2", "Xavier2");
                t.status = TaskStatus::InProgress;
                t.priority = Priority::High;
                t.labels = vec!["integration".to_string()];
                t
            },
            {
                let mut t = Task::new("Crear UI con egui", "Xavier2");
                t.status = TaskStatus::Backlog;
                t.priority = Priority::Medium;
                t.labels = vec!["ui".to_string()];
                t
            },
            {
                let mut t = Task::new("Configurar CI/CD", "Ops");
                t.status = TaskStatus::Done;
                t.priority = Priority::Low;
                t
            },
        ]
    }
}

impl eframe::App for Xavier2WebApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.is_dark_mode {
            ctx.set_visuals(Visuals::dark());
        } else {
            ctx.set_visuals(Visuals::light());
        }

        egui::SidePanel::left("sidebar")
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Xavier2 🧠");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Theme:");
                    if ui
                        .button(if self.is_dark_mode { "🌙" } else { "☀️" })
                        .clicked()
                    {
                        self.is_dark_mode = !self.is_dark_mode;
                    }
                });

                ui.separator();
                ui.label("Search:");
                ui.text_edit_singleline(&mut self.search_query);
                ui.separator();
                ui.label("Projects:");

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let all_selected = self.selected_project.is_none();
                    if ui.selectable_label(all_selected, "📋 All").clicked() {
                        self.selected_project = None;
                    }

                    for project in &self.projects {
                        let is_selected = self
                            .selected_project
                            .as_ref()
                            .map(|s| s == &project.name)
                            .unwrap_or(false);
                        let icon = project_icon(&project.name);
                        if ui
                            .selectable_label(is_selected, format!("{} {}", icon, project.name))
                            .clicked()
                        {
                            self.selected_project = Some(project.name.clone());
                        }
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let filtered_tasks: Vec<_> = self
                .tasks
                .iter()
                .filter(|t| {
                    if let Some(ref proj) = self.selected_project {
                        if &t.project != proj {
                            return false;
                        }
                    }
                    if !self.search_query.is_empty() {
                        let q = self.search_query.to_lowercase();
                        if !t.title.to_lowercase().contains(&q) {
                            return false;
                        }
                    }
                    true
                })
                .collect();

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

            egui::ScrollArea::horizontal()
                .stick_to_right(true)
                .show(ui, |ui| {
                    egui::Grid::new("kanban")
                        .num_columns(3)
                        .spacing([16.0, 0.0])
                        .show(ui, |ui| {
                            render_column(ui, "Backlog", &filtered_tasks, TaskStatus::Backlog);
                            render_column(
                                ui,
                                "In Progress",
                                &filtered_tasks,
                                TaskStatus::InProgress,
                            );
                            render_column(ui, "Done", &filtered_tasks, TaskStatus::Done);
                        });
                });
        });
    }
}

// ============================================================================
// WASM Entry Point
// ============================================================================

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();

    let app = Xavier2WebApp::default();

    // Start the web runner asynchronously
    spawn_local(async move {
        let window = web_sys::window().expect("no global window");
        let document = window.document().expect("no document");
        let canvas = document.get_element_by_id("canvas").expect("no canvas");
        let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into().expect("canvas failed");

        eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |_cc| Ok(Box::new(app))),
            )
            .await
            .expect("failed to start eframe");

        web_sys::console::log_1(&"Xavier2 Web UI ready!".into());
    });
}
