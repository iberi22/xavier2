//! TUI wizard rendering and event loop.

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;

use super::{EmbeddingProvider, InstallerState, WizardStep};

// ─── Color palette ─────────────────────────────────────────────

const ACCENT: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;
const ERROR: Color = Color::Red;
const SUCCESS: Color = Color::Green;
const BG: Color = Color::Rgb(10, 10, 20);
const CARD_BG: Color = Color::Rgb(20, 20, 35);

// ─── Public entry point ────────────────────────────────────────

pub fn run_wizard() -> Result<InstallerState> {
    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<InstallerState> {
    let mut state = InstallerState::default();

    loop {
        terminal.draw(|f| render(f, &state))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Release {
                continue;
            }
            match key.code {
                KeyCode::Char('c')
                    if key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    state.current_step = WizardStep::Done;
                    break;
                }
                KeyCode::Esc => {
                    if state.current_step == WizardStep::Welcome {
                        state.current_step = WizardStep::Done;
                        break;
                    }
                    state.current_step = state.current_step.prev();
                    state.focused_field = 0;
                    state.error_message = None;
                }
                KeyCode::Enter => handle_enter(&mut state),
                KeyCode::Tab => {
                    let count = state.field_count();
                    if count > 0 {
                        state.focused_field = (state.focused_field + 1) % count;
                    }
                }
                KeyCode::BackTab => {
                    let count = state.field_count();
                    if count > 0 {
                        state.focused_field = if state.focused_field == 0 {
                            count.saturating_sub(1)
                        } else {
                            state.focused_field - 1
                        };
                    }
                }
                KeyCode::Up => handle_up(&mut state),
                KeyCode::Down => handle_down(&mut state),
                KeyCode::Left => handle_left(&mut state),
                KeyCode::Right => handle_right(&mut state),
                KeyCode::Backspace => handle_backspace(&mut state),
                KeyCode::Delete => handle_delete(&mut state),
                KeyCode::Home => {
                    state.cursor_pos = 0;
                }
                KeyCode::End => {
                    state.cursor_pos = state.input_buffer.len();
                }
                KeyCode::Char(c) => handle_char(&mut state, c),
                _ => {}
            }
        }

        if state.current_step == WizardStep::Done {
            break;
        }
    }

    Ok(state)
}

// ─── Key handlers ──────────────────────────────────────────────

fn handle_enter(state: &mut InstallerState) {
    state.error_message = None;

    match state.current_step {
        WizardStep::Welcome => {
            state.current_step = state.current_step.next();
            state.focused_field = 0;
        }
        WizardStep::Review => {
            // Write config
            match super::config_gen::write_config(state) {
                Ok(()) => state.current_step = state.current_step.next(),
                Err(e) => state.error_message = Some(format!("Error: {}", e)),
            }
        }
        WizardStep::Done => {}
        _ => {
            // Move to next step
            let count = state.field_count();
            if state.focused_field >= count.saturating_sub(1) || count == 0 {
                // Validate before advancing
                if let Err(msg) = validate_step(state) {
                    state.error_message = Some(msg);
                    return;
                }
                state.current_step = state.current_step.next();
                state.focused_field = 0;
            } else if count > 0 {
                state.focused_field += 1;
            }
        }
    }
}

fn handle_up(state: &mut InstallerState) {
    if state.current_step == WizardStep::EmbeddingsConfig && state.focused_field == 0 {
        // Scroll provider list
        if state.provider_index > 0 {
            state.provider_index -= 1;
            state.embedding_provider = EmbeddingProvider::ALL[state.provider_index];
        }
    }
}

fn handle_down(state: &mut InstallerState) {
    if state.current_step == WizardStep::EmbeddingsConfig && state.focused_field == 0 {
        // Scroll provider list
        if state.provider_index + 1 < EmbeddingProvider::ALL.len() {
            state.provider_index += 1;
            state.embedding_provider = EmbeddingProvider::ALL[state.provider_index];
        }
    }
}

fn handle_left(state: &mut InstallerState) {
    if state.cursor_pos > 0 {
        state.cursor_pos -= 1;
    }
}

fn handle_right(state: &mut InstallerState) {
    if state.cursor_pos < state.input_buffer.len() {
        state.cursor_pos += 1;
    }
}

fn handle_backspace(state: &mut InstallerState) {
    if state.cursor_pos > 0 {
        state.cursor_pos -= 1;
        state.input_buffer.remove(state.cursor_pos);
        // Sync to state field
        let f = state.focused_field;
        let buf = state.input_buffer.clone();
        state.set_field(f, &buf);
    }
}

fn handle_delete(state: &mut InstallerState) {
    if state.cursor_pos < state.input_buffer.len() {
        state.input_buffer.remove(state.cursor_pos);
        let f = state.focused_field;
        let buf = state.input_buffer.clone();
        state.set_field(f, &buf);
    }
}

fn handle_char(state: &mut InstallerState, c: char) {
    // Filter control characters
    if c.is_control() {
        return;
    }
    state.input_buffer.insert(state.cursor_pos, c);
    state.cursor_pos += 1;
    let f = state.focused_field;
    let buf = state.input_buffer.clone();
    state.set_field(f, &buf);
}

fn validate_step(state: &InstallerState) -> Result<(), String> {
    match state.current_step {
        WizardStep::TokenSetup => {
            if state.token.trim().is_empty() {
                return Err("Token cannot be empty.".into());
            }
        }
        WizardStep::ServerConfig => {
            if state.port.parse::<u16>().is_err() {
                return Err("Port must be a number between 1 and 65535.".into());
            }
        }
        _ => {}
    }
    Ok(())
}

// ─── Rendering ─────────────────────────────────────────────────

fn render(f: &mut Frame, state: &InstallerState) {
    let area = f.area();

    // Full background
    f.render_widget(Paragraph::new("").style(Style::default().bg(BG)), area);

    // Main layout: centered card
    let card_w = (area.width.min(72)).max(40);
    let card_h = (area.height.min(22)).max(12);
    let card_x = (area.width.saturating_sub(card_w)) / 2;
    let card_y = (area.height.saturating_sub(card_h)) / 2;
    let card = Rect::new(card_x, card_y, card_w, card_h);

    // Card background
    let card_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(CARD_BG));
    f.render_widget(Clear, card);
    f.render_widget(card_block.clone(), card);

    let inner = card_block.inner(card);

    match state.current_step {
        WizardStep::Welcome => render_welcome(f, inner, state),
        WizardStep::TokenSetup => render_token_setup(f, inner, state),
        WizardStep::ServerConfig => render_server_config(f, inner, state),
        WizardStep::StorageConfig => render_storage_config(f, inner, state),
        WizardStep::EmbeddingsConfig => render_embeddings_config(f, inner, state),
        WizardStep::Review => render_review(f, inner, state),
        WizardStep::Done => render_done(f, inner, state),
    }
}

// ─── Shared helpers ────────────────────────────────────────────

fn title_line<'a>(text: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled("█ ", Style::default().fg(ACCENT)),
        Span::styled(
            text,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

fn footer_hint(text: &str) -> Line<'_> {
    Line::from(Span::styled(text, Style::default().fg(DIM)))
}

fn progress_bar(steps: &[WizardStep], current: WizardStep) -> Line<'_> {
    let mut spans = Vec::new();
    for (i, step) in steps.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" · "));
        }
        if *step == current {
            spans.push(Span::styled("●".to_string(), Style::default().fg(ACCENT)));
        } else if step_order(*step) < step_order(current) {
            spans.push(Span::styled("●", Style::default().fg(SUCCESS)));
        } else {
            spans.push(Span::styled("○", Style::default().fg(DIM)));
        }
    }
    Line::from(spans)
}

fn step_order(step: WizardStep) -> u8 {
    match step {
        WizardStep::Welcome => 0,
        WizardStep::TokenSetup => 1,
        WizardStep::ServerConfig => 2,
        WizardStep::StorageConfig => 3,
        WizardStep::EmbeddingsConfig => 4,
        WizardStep::Review => 5,
        WizardStep::Done => 6,
    }
}

const ALL_STEPS: &[WizardStep] = &[
    WizardStep::Welcome,
    WizardStep::TokenSetup,
    WizardStep::ServerConfig,
    WizardStep::StorageConfig,
    WizardStep::EmbeddingsConfig,
    WizardStep::Review,
];

/// Render a text input field with label, value, and cursor.
pub(crate) fn render_input_field(
    f: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    focused: bool,
    cursor_pos: usize,
    masked: bool,
) {
    // Helper: convert char-position cursor to byte offset for slicing
    let char_to_byte = |s: &str, cp: usize| -> usize {
        s.char_indices().nth(cp).map(|(i, _)| i).unwrap_or(s.len())
    };

    let display_value = if masked && !value.is_empty() {
        "•".repeat(value.chars().count())
    } else {
        value.to_string()
    };

    let label_text = format!("{}: ", label);

    let label_style = if focused {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    let mut spans = vec![Span::styled(label_text, label_style)];

    if focused {
        // Show cursor
        let byte_cursor = char_to_byte(&display_value, cursor_pos);
        if byte_cursor < display_value.len() {
            spans.push(Span::styled(
                display_value[..byte_cursor].to_string(),
                Style::default().fg(Color::White),
            ));
            spans.push(Span::styled(
                display_value
                    .chars()
                    .nth(cursor_pos)
                    .unwrap_or(' ')
                    .to_string(),
                Style::default().fg(Color::Black).bg(ACCENT),
            ));
            let byte_cursor_next = char_to_byte(&display_value, cursor_pos + 1);
            if byte_cursor_next < display_value.len() {
                spans.push(Span::styled(
                    display_value[byte_cursor_next..].to_string(),
                    Style::default().fg(Color::White),
                ));
            }
        } else {
            spans.push(Span::styled(
                display_value,
                Style::default().fg(Color::White),
            ));
            spans.push(Span::styled(
                " ",
                Style::default().fg(Color::Black).bg(ACCENT),
            ));
        }
    } else {
        spans.push(Span::styled(
            if display_value.is_empty() {
                "(empty)"
            } else {
                &display_value
            },
            Style::default().fg(if display_value.is_empty() {
                DIM
            } else {
                Color::White
            }),
        ));
    }

    let para = Paragraph::new(Line::from(spans));
    f.render_widget(para, area);
}

// ─── Step renderers ────────────────────────────────────────────

fn render_welcome(f: &mut Frame, area: Rect, _state: &InstallerState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(4),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    f.render_widget(Paragraph::new(title_line(" Welcome to Xavier2")), chunks[0]);

    // Logo
    let logo = vec![
        Line::from(Span::styled(
            r"  ██╗  ██╗ █████╗ ██╗   ██╗██╗███████╗██████╗ ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            r"  ╚██╗██╔╝██╔══██╗██║   ██║██║██╔════╝██╔══██╗",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            r"   ╚███╔╝ ███████║██║   ██║██║█████╗  ██████╔╝",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            r"   ██╔██╗ ██╔══██║╚██╗ ██╔╝██║██╔══╝  ██╔══██╗",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            r"  ██╔╝ ██╗██║  ██║ ╚████╔╝ ██║███████╗██║  ██║",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            r"  ╚═╝  ╚═╝╚═╝  ╚═╝  ╚═══╝  ╚═╝╚══════╝╚═╝  ╚═╝",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
    ];
    f.render_widget(Paragraph::new(logo).alignment(Alignment::Center), chunks[1]);

    f.render_widget(
        Paragraph::new("Cognitive Memory Runtime for AI Agents")
            .style(Style::default().fg(DIM))
            .alignment(Alignment::Center),
        chunks[3],
    );

    f.render_widget(
        Paragraph::new("This wizard will guide you through initial setup.\nAll settings are saved to config/xavier2.config.json")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        chunks[4],
    );

    f.render_widget(
        Paragraph::new(footer_hint("Press ENTER to begin · Esc to quit"))
            .alignment(Alignment::Center),
        chunks[6],
    );
}

fn render_token_setup(f: &mut Frame, area: Rect, state: &InstallerState) {
    let chunks = standard_step_layout(area);

    f.render_widget(
        Paragraph::new(title_line(state.current_step.title())),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(progress_bar(ALL_STEPS, state.current_step)),
        chunks[1],
    );

    f.render_widget(
        Paragraph::new("Set a local token for API authentication.\nThis token is required when calling Xavier2 endpoints.")
            .style(Style::default().fg(DIM)),
        chunks[2],
    );

    // Input field
    let input_area = Rect::new(chunks[3].x, chunks[3].y, chunks[3].width.min(50), 1);
    let focused = state.focused_field == 0;
    if focused {
        // Copy field value to buffer on focus
        // (We handle this via implicit sync since buffer is pre-filled)
    }
    render_input_field(
        f,
        input_area,
        state.field_label(0),
        &state.token,
        focused,
        state.cursor_pos,
        state.field_masked(0),
    );

    // Error
    if let Some(ref err) = state.error_message {
        f.render_widget(
            Paragraph::new(Span::styled(err, Style::default().fg(ERROR))),
            chunks[5],
        );
    }

    f.render_widget(
        Paragraph::new(footer_hint(
            "Type to edit · Enter to confirm · Tab to change field · Esc to go back",
        ))
        .alignment(Alignment::Left),
        chunks[7],
    );
}

fn render_server_config(f: &mut Frame, area: Rect, state: &InstallerState) {
    let chunks = standard_step_layout(area);

    f.render_widget(
        Paragraph::new(title_line(state.current_step.title())),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(progress_bar(ALL_STEPS, state.current_step)),
        chunks[1],
    );

    f.render_widget(
        Paragraph::new("Configure the server binding.\nHost localhost = local only · 0.0.0.0 = accessible on network")
            .style(Style::default().fg(DIM)),
        chunks[2],
    );

    // Field 0: Host
    let input_area = Rect::new(chunks[3].x, chunks[3].y, chunks[3].width.min(50), 1);
    render_input_field(
        f,
        input_area,
        state.field_label(0),
        &state.host,
        state.focused_field == 0,
        if state.focused_field == 0 {
            state.cursor_pos
        } else {
            0
        },
        false,
    );

    // Field 1: Port
    let port_area = Rect::new(chunks[3].x, chunks[3].y + 2, chunks[3].width.min(50), 1);
    render_input_field(
        f,
        port_area,
        state.field_label(1),
        &state.port,
        state.focused_field == 1,
        if state.focused_field == 1 {
            state.cursor_pos
        } else {
            0
        },
        false,
    );

    if let Some(ref err) = state.error_message {
        f.render_widget(
            Paragraph::new(Span::styled(err, Style::default().fg(ERROR))),
            chunks[5],
        );
    }

    f.render_widget(
        Paragraph::new(footer_hint(
            "Type to edit · Enter to confirm · Tab/Shift+Tab to switch field · Esc to go back",
        ))
        .alignment(Alignment::Left),
        chunks[7],
    );
}

fn render_storage_config(f: &mut Frame, area: Rect, state: &InstallerState) {
    let chunks = standard_step_layout(area);

    f.render_widget(
        Paragraph::new(title_line(state.current_step.title())),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(progress_bar(ALL_STEPS, state.current_step)),
        chunks[1],
    );

    f.render_widget(
        Paragraph::new("Where should Xavier2 store its data?\nMemory, vector indexes, and workspace files live here.\nDefault: data/")
            .style(Style::default().fg(DIM)),
        chunks[2],
    );

    let input_area = Rect::new(chunks[3].x, chunks[3].y, chunks[3].width.min(50), 1);
    render_input_field(
        f,
        input_area,
        state.field_label(0),
        &state.data_dir,
        state.focused_field == 0,
        if state.focused_field == 0 {
            state.cursor_pos
        } else {
            0
        },
        false,
    );

    // Preview of paths
    let preview = format!(
        "  {}workspaces/\n  {}memory-store.sqlite3\n  {}vec-store.sqlite3\n  {}code_graph.db",
        state.data_dir, state.data_dir, state.data_dir, state.data_dir
    );
    f.render_widget(
        Paragraph::new(preview).style(Style::default().fg(DIM)),
        chunks[4],
    );

    f.render_widget(
        Paragraph::new(footer_hint(
            "Type to edit · Enter to confirm · Esc to go back",
        ))
        .alignment(Alignment::Left),
        chunks[7],
    );
}

fn render_embeddings_config(f: &mut Frame, area: Rect, state: &InstallerState) {
    let chunks = standard_step_layout(area);

    f.render_widget(
        Paragraph::new(title_line(state.current_step.title())),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(progress_bar(ALL_STEPS, state.current_step)),
        chunks[1],
    );

    f.render_widget(
        Paragraph::new("Choose your embedding provider. Xavier2 uses embeddings for semantic search.\n↑↓ to select · Enter to confirm.")
            .style(Style::default().fg(DIM)),
        chunks[2],
    );

    // Provider list
    let items: Vec<ListItem> = EmbeddingProvider::ALL
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let style = if i == state.provider_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(Line::from(Span::styled(format!(" ▶ {}", p.label()), style)))
        })
        .collect();

    let list_area = Rect::new(chunks[3].x, chunks[3].y, chunks[3].width.min(55), 5);
    let list = List::new(items)
        .block(Block::default().borders(Borders::NONE))
        .highlight_style(Style::default().bg(ACCENT));
    f.render_widget(list, list_area);

    // Description
    if let Some(provider) = EmbeddingProvider::ALL.get(state.provider_index) {
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("ℹ ", Style::default().fg(ACCENT)),
                Span::styled(provider.description(), Style::default().fg(DIM)),
            ]))
            .wrap(Wrap { trim: true }),
            chunks[4],
        );
    }

    // Conditional fields for Ollama / OpenAI
    let base = chunks[5];
    match state.embedding_provider {
        EmbeddingProvider::Ollama => {
            let model_area = Rect::new(base.x, base.y, base.width.min(50), 1);
            render_input_field(
                f,
                model_area,
                "Model Name",
                &state.embedding_model,
                state.focused_field == 1,
                if state.focused_field == 1 {
                    state.cursor_pos
                } else {
                    0
                },
                false,
            );
            let url_area = Rect::new(base.x, base.y + 2, base.width.min(50), 1);
            render_input_field(
                f,
                url_area,
                "API URL",
                &state.embedding_url,
                state.focused_field == 2,
                if state.focused_field == 2 {
                    state.cursor_pos
                } else {
                    0
                },
                false,
            );
        }
        EmbeddingProvider::OpenAI => {
            let model_area = Rect::new(base.x, base.y, base.width.min(50), 1);
            render_input_field(
                f,
                model_area,
                "Model Name",
                &state.embedding_model,
                state.focused_field == 1,
                if state.focused_field == 1 {
                    state.cursor_pos
                } else {
                    0
                },
                false,
            );
            let url_area = Rect::new(base.x, base.y + 2, base.width.min(50), 1);
            render_input_field(
                f,
                url_area,
                "API URL",
                &state.embedding_url,
                state.focused_field == 2,
                if state.focused_field == 2 {
                    state.cursor_pos
                } else {
                    0
                },
                false,
            );
            let key_area = Rect::new(base.x, base.y + 4, base.width.min(50), 1);
            render_input_field(
                f,
                key_area,
                "API Key",
                &state.api_key,
                state.focused_field == 3,
                if state.focused_field == 3 {
                    state.cursor_pos
                } else {
                    0
                },
                true,
            );
        }
        _ => {}
    }

    f.render_widget(
        Paragraph::new(footer_hint(
            "↑↓ select provider · Tab/Shift+Tab switch field · Enter to confirm · Esc to go back",
        ))
        .alignment(Alignment::Left),
        chunks[7],
    );
}

fn render_review(f: &mut Frame, area: Rect, state: &InstallerState) {
    let chunks = standard_step_layout(area);

    f.render_widget(
        Paragraph::new(title_line(state.current_step.title())),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(progress_bar(ALL_STEPS, state.current_step)),
        chunks[1],
    );

    let config = super::config_gen::generate_config(state);
    let json = serde_json::to_string_pretty(&config).unwrap_or_default();

    // Show a preview of the config
    let preview_lines: Vec<&str> = json.lines().take(10).collect();
    let preview = preview_lines.join("\n");

    f.render_widget(
        Paragraph::new("Configuration to be written:\n").style(Style::default().fg(Color::White)),
        chunks[2],
    );

    f.render_widget(
        Paragraph::new(format!(
            "{}\n  ... ({} total lines)",
            preview,
            json.lines().count()
        ))
        .style(Style::default().fg(DIM)),
        chunks[3],
    );

    if state.config_written {
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("✓ Config saved to {}", state.config_path),
                Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD),
            )),
            chunks[5],
        );
    } else if let Some(ref err) = state.error_message {
        f.render_widget(
            Paragraph::new(Span::styled(err, Style::default().fg(ERROR))),
            chunks[5],
        );
    }

    f.render_widget(
        Paragraph::new(footer_hint("Enter to save config · Esc to go back"))
            .alignment(Alignment::Left),
        chunks[7],
    );
}

fn render_done(f: &mut Frame, area: Rect, _state: &InstallerState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Min(2),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new(title_line(" Setup Complete!")).style(Style::default().fg(SUCCESS)),
        chunks[0],
    );

    f.render_widget(
        Paragraph::new("✓ Configuration saved successfully")
            .style(Style::default().fg(SUCCESS))
            .alignment(Alignment::Center),
        chunks[2],
    );

    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "Next steps:",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  xavier serve",
                Style::default().fg(Color::White),
            )),
            Line::from(Span::styled(
                "    Start the Xavier2 memory server",
                Style::default().fg(DIM),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  xavier tui",
                Style::default().fg(Color::White),
            )),
            Line::from(Span::styled(
                "    Launch the monitoring dashboard",
                Style::default().fg(DIM),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  xavier save --kind episodic \"memory text\"",
                Style::default().fg(Color::White),
            )),
            Line::from(Span::styled(
                "    Save your first memory",
                Style::default().fg(DIM),
            )),
        ])
        .alignment(Alignment::Left),
        chunks[3],
    );

    f.render_widget(
        Paragraph::new(footer_hint("Press any key to exit")).alignment(Alignment::Center),
        chunks[5],
    );
}

// ─── Layout helpers ────────────────────────────────────────────

fn standard_step_layout(area: Rect) -> std::rc::Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title
            Constraint::Length(1), // progress dots
            Constraint::Length(2), // description
            Constraint::Length(1), // input field(s)
            Constraint::Length(2), // secondary info
            Constraint::Length(1), // status/message
            Constraint::Min(1),    // spacer
            Constraint::Length(1), // footer
        ])
        .split(area)
}

// ─── Screenshot / documentation rendering ─────────────────────

/// Render a single wizard step to styled ANSI text.
/// Uses ratatui's TestBackend to render into a buffer,
/// then converts buffer cells to ANSI SGR escape codes.
#[cfg(feature = "cli-interactive")]
pub fn render_step_ansi(step: super::WizardStep, state: &super::InstallerState) -> Result<String> {
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Cell;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;
    let mut s = state.clone();
    s.current_step = step;

    // Pre-fill for visual interest
    if step == super::WizardStep::TokenSetup {
        s.input_buffer = s.token.clone();
    }
    if step == super::WizardStep::ServerConfig {
        s.focused_field = 1;
    }

    terminal.draw(|f| render(f, &s))?;

    let buf = terminal.backend().buffer();
    let area = *buf.area();
    let mut out = String::new();
    let mut last_fg = Color::Reset;
    let mut last_bg = Color::Reset;
    let mut last_bold = false;

    for row in 0..area.height {
        for col in 0..area.width {
            let cell: &Cell = &buf[(col, row)];
            let ch = cell.symbol();
            let cur_fg = cell.fg;
            let cur_bg = cell.bg;
            let cur_bold = cell.modifier.contains(Modifier::BOLD);

            // Emit SGR when style changes
            if cur_fg != last_fg || cur_bg != last_bg || cur_bold != last_bold {
                out.push_str("\u{1b}[0m");
                let mut codes = Vec::new();
                if cur_bold {
                    codes.push("1".to_string());
                }
                codes.push(color_to_ansi_fg(cur_fg));
                if cur_bg != Color::Reset {
                    codes.push(color_to_ansi_bg(cur_bg));
                }
                out.push_str(&format!("\u{1b}[{}m", codes.join(";")));
                last_fg = cur_fg;
                last_bg = cur_bg;
                last_bold = cur_bold;
            }

            out.push_str(ch);
        }
        out.push('\n');
    }
    out.push_str("\u{1b}[0m");

    Ok(out)
}

fn color_to_ansi_fg(color: Color) -> String {
    match color {
        Color::Reset => "39".into(),
        Color::Black => "30".into(),
        Color::Red => "31".into(),
        Color::Green => "32".into(),
        Color::Yellow => "33".into(),
        Color::Blue => "34".into(),
        Color::Magenta => "35".into(),
        Color::Cyan => "36".into(),
        Color::White | Color::Gray => "37".into(),
        Color::DarkGray => "90".into(),
        Color::LightRed => "91".into(),
        Color::LightGreen => "92".into(),
        Color::LightYellow => "93".into(),
        Color::LightBlue => "94".into(),
        Color::LightMagenta => "95".into(),
        Color::LightCyan => "96".into(),
        Color::Rgb(r, g, b) => format!("38;2;{};{};{}", r, g, b),
        _ => "39".into(),
    }
}

fn color_to_ansi_bg(color: Color) -> String {
    match color {
        Color::Reset => "49".into(),
        Color::Black => "40".into(),
        Color::Red => "41".into(),
        Color::Green => "42".into(),
        Color::Yellow => "43".into(),
        Color::Blue => "44".into(),
        Color::Magenta => "45".into(),
        Color::Cyan => "46".into(),
        Color::White | Color::Gray => "47".into(),
        Color::DarkGray => "100".into(),
        Color::Rgb(r, g, b) => format!("48;2;{};{};{}", r, g, b),
        _ => "49".into(),
    }
}

/// Render all wizard steps to `.ansi` files for documentation.
#[cfg(feature = "cli-interactive")]
pub fn render_all_steps_ansi(out_dir: &str) -> Result<Vec<String>> {
    use std::fs;
    use std::io::Write;

    fs::create_dir_all(out_dir)?;

    let steps = [
        super::WizardStep::Welcome,
        super::WizardStep::TokenSetup,
        super::WizardStep::ServerConfig,
        super::WizardStep::StorageConfig,
        super::WizardStep::EmbeddingsConfig,
        super::WizardStep::Review,
    ];
    let names = [
        "welcome",
        "token",
        "server",
        "storage",
        "embeddings",
        "review",
    ];

    let state = super::InstallerState::default();
    let mut files = Vec::new();

    for (&step, name) in steps.iter().zip(names.iter()) {
        let ansi = render_step_ansi(step, &state)?;
        let path = std::path::Path::new(out_dir).join(format!("{}.ansi", name));
        let mut f = fs::File::create(&path)?;
        f.write_all(ansi.as_bytes())?;
        files.push(path.to_string_lossy().to_string());
    }

    Ok(files)
}
