//! Xavier2 Dashboard - Modern Web UI for Production
//!
//! This module provides a production-ready web interface for Xavier2.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Tabs},
    Frame,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Dashboard metrics for real-time display
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DashboardMetrics {
    pub total_memories: usize,
    pub total_beliefs: usize,
    pub active_agents: usize,
    pub queries_today: usize,
    pub avg_response_time_ms: u64,
    pub uptime_seconds: u64,
    pub storage_used: u64,
    pub storage_limit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub content: String,
    pub path: String,
    pub metadata: serde_json::Value,
}

/// System status for health checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub status: StatusLevel,
    pub version: String,
    pub database_connected: bool,
    pub mcp_server_running: bool,
    pub last_backup: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StatusLevel {
    Healthy,
    Warning,
    Error,
}

impl Default for SystemStatus {
    fn default() -> Self {
        Self {
            status: StatusLevel::Healthy,
            version: env!("CARGO_PKG_VERSION").to_string(),
            database_connected: false,
            mcp_server_running: false,
            last_backup: None,
        }
    }
}

pub trait TuiAppState {
    fn metrics(&self) -> &DashboardMetrics;
    fn memories(&self) -> &[MemoryItem];
    fn current_tab(&self) -> usize;
    fn memory_state(&self) -> &ratatui::widgets::TableState;
    fn logs(&self) -> &[String];
}

pub fn render_tui<S: TuiAppState>(f: &mut Frame, app: &S) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(f.area());

    let titles = vec!["Dashboard", "Memory Explorer", "Log Stream"];
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Xavier2 Monitor"),
        )
        .select(app.current_tab())
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Black),
        );
    f.render_widget(tabs, chunks[0]);

    match app.current_tab() {
        0 => render_stats(f, app.metrics(), chunks[1]),
        1 => {
            let mut memory_view = crate::ui::memory_view::MemoryView::new();
            memory_view.state = app.memory_state().clone();
            memory_view.render(f, app.memories(), chunks[1]);
        }
        2 => {
            let mut log_stream = crate::ui::log_stream::LogStream::new();
            log_stream.render(f, app.logs(), chunks[1]);
        }
        _ => {}
    }
}

fn render_stats(f: &mut Frame, metrics: &DashboardMetrics, area: Rect) {
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(area);

    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(vertical_chunks[0]);

    let stats = vec![
        Line::from(vec![
            Span::raw("Total Memories: "),
            Span::styled(
                metrics.total_memories.to_string(),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::raw("Queries Today: "),
            Span::styled(
                metrics.queries_today.to_string(),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::raw("Active Agents: "),
            Span::styled(
                metrics.active_agents.to_string(),
                Style::default().fg(Color::Magenta),
            ),
        ]),
    ];

    let p = Paragraph::new(stats)
        .block(Block::default().borders(Borders::ALL).title("Statistics"))
        .style(Style::default().fg(Color::White));
    f.render_widget(p, horizontal_chunks[0]);

    let help = vec![
        Line::from("Navigation:"),
        Line::from("  Tab: Switch tabs"),
        Line::from("  q: Quit"),
        Line::from(""),
        Line::from("Keybindings:"),
        Line::from("  j/k: Scroll down/up"),
        Line::from("  Enter: View details"),
        Line::from("  /: Search (Memory Explorer only)"),
    ];
    let p_help = Paragraph::new(help)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .style(Style::default().fg(Color::Gray));
    f.render_widget(p_help, horizontal_chunks[1]);

    if metrics.storage_limit > 0 {
        let ratio = metrics.storage_used as f64 / metrics.storage_limit as f64;
        let label = format!(
            "Storage Usage: {:.2}% ({}/{})",
            ratio * 100.0,
            format_bytes(metrics.storage_used),
            format_bytes(metrics.storage_limit)
        );
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Quotas"))
            .gauge_style(
                Style::default()
                    .fg(Color::Blue)
                    .bg(Color::Black)
                    .add_modifier(Modifier::ITALIC),
            )
            .ratio(ratio.min(1.0))
            .label(label);
        f.render_widget(gauge, vertical_chunks[1]);
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

/// Memory item for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDisplay {
    pub id: String,
    pub content: String,
    pub path: String,
    pub created_at: String,
    pub tags: Vec<String>,
}

impl MemoryDisplay {
    pub fn preview(&self, max_len: usize) -> String {
        if self.content.len() > max_len {
            format!("{}...", &self.content[..max_len])
        } else {
            self.content.clone()
        }
    }
}

/// Belief node for graph visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefDisplay {
    pub id: String,
    pub label: String,
    pub belief_type: String,
    pub confidence: f32,
    pub connections: Vec<String>,
}

/// Search result with highlighting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultDisplay {
    pub id: String,
    pub content: String,
    pub path: String,
    pub score: f32,
    pub highlights: Vec<String>,
}

/// User preferences for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub theme: Theme,
    pub language: String,
    pub sidebar_collapsed: bool,
    pub notifications_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
    System,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            language: "en".to_string(),
            sidebar_collapsed: false,
            notifications_enabled: true,
        }
    }
}

/// API Response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}

/// Pagination for list responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
    pub has_next: bool,
    pub has_prev: bool,
}

impl<T> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, total: usize, page: usize, per_page: usize) -> Self {
        Self {
            has_next: page * per_page < total,
            has_prev: page > 1,
            total,
            page,
            per_page,
            items,
        }
    }
}

/// WebSocket message for real-time updates
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsMessage {
    MemoryAdded(MemoryDisplay),
    MemoryDeleted(String),
    BeliefUpdated(BeliefDisplay),
    MetricsUpdated(DashboardMetrics),
    TaskStatusChanged { task_id: String, status: String },
    AgentConnected { agent_id: String },
    AgentDisconnected { agent_id: String },
}

/// Configuration for the web UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebUIConfig {
    pub title: String,
    pub logo_url: Option<String>,
    pub primary_color: String,
    pub accent_color: String,
    pub features: HashMap<String, bool>,
}

impl Default for WebUIConfig {
    fn default() -> Self {
        let mut features = HashMap::new();
        features.insert("belief_graph".to_string(), true);
        features.insert("kanban".to_string(), true);
        features.insert("analytics".to_string(), true);
        features.insert("api_docs".to_string(), true);

        Self {
            title: "Xavier2".to_string(),
            logo_url: None,
            primary_color: "#6366f1".to_string(),
            accent_color: "#818cf8".to_string(),
            features,
        }
    }
}
