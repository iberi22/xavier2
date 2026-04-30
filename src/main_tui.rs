use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    io,
    sync::Arc,
    time::{Duration, Instant},
};

use xavier2::ui::dashboard::{DashboardMetrics, MemoryItem, TuiAppState};
use xavier2::ui::memory_view::MemoryView;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryListResponse {
    pub memories: Vec<MemoryItem>,
}

pub struct Xavier2Client {
    client: Client,
    base_url: String,
    token: String,
}

impl Default for Xavier2Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Xavier2Client {
    pub fn new() -> Self {
        let base_url =
            std::env::var("XAVIER2_URL").unwrap_or_else(|_| "http://localhost:8003".to_string());
        let token = std::env::var("XAVIER2_TOKEN").expect("XAVIER2_TOKEN environment variable must be set");
        Self {
            client: Client::new(),
            base_url,
            token,
        }
    }

    pub async fn get_metrics(&self) -> Result<DashboardMetrics> {
        let url = format!("{}/v1/account/usage", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header("X-Xavier2-Token", &self.token)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let metrics = DashboardMetrics {
            total_memories: resp["document_count"].as_u64().unwrap_or(0) as usize,
            total_beliefs: 0,
            active_agents: 1,
            queries_today: resp["requests_used"].as_u64().unwrap_or(0) as usize,
            avg_response_time_ms: 0,
            uptime_seconds: 0,
            storage_used: resp["storage_bytes_used"].as_u64().unwrap_or(0),
            storage_limit: resp["storage_bytes_limit"].as_u64().unwrap_or(0),
        };
        Ok(metrics)
    }

    pub async fn get_memories(&self) -> Result<Vec<MemoryItem>> {
        let url = format!("{}/v1/memories", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header("X-Xavier2-Token", &self.token)
            .send()
            .await?
            .json::<MemoryListResponse>()
            .await?;
        Ok(resp.memories)
    }

    pub async fn search_memories(&self, query: &str) -> Result<Vec<MemoryItem>> {
        let url = format!("{}/v1/memories/search", self.base_url);
        let resp = self
            .client
            .post(&url)
            .header("X-Xavier2-Token", &self.token)
            .json(&serde_json::json!({ "query": query, "limit": 50 }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let results = resp["results"].as_array().cloned().unwrap_or_default();
        let memories = results
            .into_iter()
            .filter_map(|v| serde_json::from_value::<MemoryItem>(v).ok())
            .collect();
        Ok(memories)
    }

    pub async fn get_logs(&self) -> Result<Vec<String>> {
        let url = format!("{}/panel/api/threads", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header("X-Xavier2-Token", &self.token)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let mut logs = Vec::new();
        if let Some(threads) = resp.as_array() {
            for thread in threads.iter().take(20) {
                let title = thread["title"].as_str().unwrap_or("Untitled");
                let id = thread["id"].as_str().unwrap_or("?");
                logs.push(format!("Activity: Thread '{}' ({})", title, id));
            }
        }

        if logs.is_empty() {
            logs = vec![
                "S1: [Retrieval] Searching memory for 'xavier2 architecture'...".to_string(),
                "S1: Found 3 relevant documents.".to_string(),
                "S2: [Reasoning] Analyzing system context and retrieved docs...".to_string(),
                "S2: Confidence 0.85. Reasoning chain: memory-indexing -> agent-access."
                    .to_string(),
                "S3: [Action] Generating response for user query.".to_string(),
                "S3: Response delivered (245ms).".to_string(),
            ];
        }

        Ok(logs)
    }
}

pub struct TuiApp {
    pub metrics: DashboardMetrics,
    pub memories: Vec<MemoryItem>,
    pub logs: Vec<String>,
    pub current_tab: usize,
    pub should_quit: bool,
    pub client: Arc<Xavier2Client>,
    pub last_tick: Instant,
    pub memory_view: MemoryView,
    pub input: String,
    pub input_mode: InputMode,
    pub selected_memory: Option<MemoryItem>,
}

pub enum InputMode {
    Normal,
    Editing,
    ViewingDetails,
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}

impl TuiApp {
    pub fn new() -> Self {
        let mut app = Self {
            metrics: DashboardMetrics::default(),
            memories: Vec::new(),
            logs: Vec::new(),
            current_tab: 0,
            should_quit: false,
            client: Arc::new(Xavier2Client::new()),
            last_tick: Instant::now(),
            memory_view: MemoryView::new(),
            input: String::new(),
            input_mode: InputMode::Normal,
            selected_memory: None,
        };
        app.memory_view.state.select(Some(0));
        app
    }

    pub async fn on_tick(&mut self) {
        if matches!(self.input_mode, InputMode::Normal) {
            if let Ok(m) = self.client.get_metrics().await {
                self.metrics = m;
            }
            if let Ok(mem) = self.client.get_memories().await {
                self.memories = mem;
            }
            if let Ok(l) = self.client.get_logs().await {
                self.logs = l;
            }
        }
    }

    pub async fn execute_search(&mut self) {
        if let Ok(mem) = self.client.search_memories(&self.input).await {
            self.memories = mem;
            self.input_mode = InputMode::Normal;
            self.input.clear();
            self.memory_view.state.select(Some(0));
        }
    }
}

impl TuiAppState for TuiApp {
    fn metrics(&self) -> &DashboardMetrics {
        &self.metrics
    }
    fn memories(&self) -> &[MemoryItem] {
        &self.memories
    }
    fn current_tab(&self) -> usize {
        self.current_tab
    }
    fn memory_state(&self) -> &ratatui::widgets::TableState {
        &self.memory_view.state
    }
    fn logs(&self) -> &[String] {
        &self.logs
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run loop
    let mut app = TuiApp::new();
    let tick_rate = Duration::from_millis(250);

    loop {
        terminal.draw(|f| {
            xavier2::ui::dashboard::render_tui(f, &app);

            if matches!(app.input_mode, InputMode::Editing) {
                let area = f.area();
                let layout = ratatui::layout::Layout::default()
                    .direction(ratatui::layout::Direction::Vertical)
                    .constraints([
                        ratatui::layout::Constraint::Min(0),
                        ratatui::layout::Constraint::Length(3),
                    ])
                    .split(area);
                let input = ratatui::widgets::Paragraph::new(app.input.as_str())
                    .style(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow))
                    .block(
                        ratatui::widgets::Block::default()
                            .borders(ratatui::widgets::Borders::ALL)
                            .title("Search Memory (Enter to search, Esc to cancel)"),
                    );
                f.render_widget(input, layout[1]);
            } else if matches!(app.input_mode, InputMode::ViewingDetails) {
                if let Some(ref mem) = app.selected_memory {
                    let area = f.area();
                    let popup_area = ratatui::layout::Rect::new(
                        area.width / 10,
                        area.height / 10,
                        area.width * 8 / 10,
                        area.height * 8 / 10,
                    );
                    let content = format!(
                        "ID: {}\nPath: {}\n\nContent:\n{}\n\nMetadata:\n{}",
                        mem.id,
                        mem.path,
                        mem.content,
                        serde_json::to_string_pretty(&mem.metadata).unwrap_or_default()
                    );
                    let p = ratatui::widgets::Paragraph::new(content)
                        .block(
                            ratatui::widgets::Block::default()
                                .borders(ratatui::widgets::Borders::ALL)
                                .title("Memory Details (Esc to close)"),
                        )
                        .wrap(ratatui::widgets::Wrap { trim: true })
                        .style(ratatui::style::Style::default().bg(ratatui::style::Color::Black));
                    f.render_widget(ratatui::widgets::Clear, popup_area);
                    f.render_widget(p, popup_area);
                }
            }
        })?;

        let timeout = tick_rate
            .checked_sub(app.last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => app.should_quit = true,
                        KeyCode::Tab => {
                            app.current_tab = (app.current_tab + 1) % 3;
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            if app.current_tab == 1 && !app.memories.is_empty() {
                                app.memory_view.next(app.memories.len());
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if app.current_tab == 1 && !app.memories.is_empty() {
                                app.memory_view.previous(app.memories.len());
                            }
                        }
                        KeyCode::Enter => {
                            if app.current_tab == 1 {
                                if let Some(selected) = app.memory_view.state.selected() {
                                    if let Some(mem) = app.memories.get(selected) {
                                        app.selected_memory = Some(mem.clone());
                                        app.input_mode = InputMode::ViewingDetails;
                                    }
                                }
                            }
                        }
                        KeyCode::Char('/') => {
                            if app.current_tab == 1 {
                                app.input_mode = InputMode::Editing;
                            }
                        }
                        _ => {}
                    },
                    InputMode::Editing => match key.code {
                        KeyCode::Enter => {
                            app.execute_search().await;
                        }
                        KeyCode::Char(c) => {
                            app.input.push(c);
                        }
                        KeyCode::Backspace => {
                            app.input.pop();
                        }
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                            app.input.clear();
                        }
                        _ => {}
                    },
                    InputMode::ViewingDetails => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.input_mode = InputMode::Normal;
                            app.selected_memory = None;
                        }
                        _ => {}
                    },
                }
            }
        }

        if app.last_tick.elapsed() >= tick_rate {
            app.on_tick().await;
            app.last_tick = Instant::now();
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
