//! UI Rendering
//!
//! Main UI layout and rendering logic for the TUI.

use crate::tui::app::{App, ApiStatus, MessageRole, PipelineStage, View};
use crate::tui::theme::{Icons, Theme};
use crate::tui::widgets;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

const SPINNER_FRAMES: [&str; 4] = ["-", "\\", "|", "/"];

/// Render the main UI
pub fn render(frame: &mut Frame, app: &App) {
    // Main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(4),  // Progress
            Constraint::Min(10),    // Messages
            Constraint::Length(4),  // Input
            Constraint::Length(1),  // Status bar
        ])
        .split(frame.area());

    render_header(frame, chunks[0], app);
    widgets::render_progress(frame, chunks[1], &app.pipeline_stage, &app.current_objective);
    render_messages(frame, chunks[2], app);
    render_input(frame, chunks[3], app);
    render_status_bar(frame, chunks[4], app);

    // Render modal overlays
    match app.view {
        View::Settings => widgets::render_settings(frame, app),
        View::Help => render_help(frame),
        View::Chat => {}
    }
}

/// Render the header with API status indicators
fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    // Status dot styles
    let green_dot = Style::default().fg(Color::Green);
    let red_dot = Style::default().fg(Color::Red);

    // LLM status dot
    let llm_dot = match app.llm_status {
        ApiStatus::Ready => Span::styled("●", green_dot),
        ApiStatus::NotConfigured => Span::styled("●", red_dot),
    };

    // Search (SerpAPI) status dot  
    let search_dot = match app.search_status {
        ApiStatus::Ready => Span::styled("●", green_dot),
        ApiStatus::NotConfigured => Span::styled("●", red_dot),
    };

    let spinner = if matches!(app.pipeline_stage, PipelineStage::Generating) {
        let idx = app.spinner_index % SPINNER_FRAMES.len();
        Span::styled(SPINNER_FRAMES[idx], Theme::text_dim())
    } else {
        Span::raw(" ")
    };

    let title_text = vec![Line::from(vec![
        Span::styled("Oxidized Bio", Theme::title()),
        Span::styled(" Research Agent", Theme::text_secondary()),
        Span::raw("  "),
        llm_dot,
        Span::raw(" "),
        search_dot,
        Span::raw(" "),
        spinner,
    ])];

    let title = Paragraph::new(title_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Theme::border())
                .style(Style::default()),
        );

    frame.render_widget(title, area);
}

/// Render the message history
fn render_messages(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Messages ")
        .borders(Borders::ALL)
        .border_style(if app.view == View::Chat {
            Theme::border_focused()
        } else {
            Theme::border()
        });

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Build message lines
    let mut lines: Vec<Line> = Vec::new();
    let available_width = inner_area.width.saturating_sub(2) as usize; // Leave room for indent

    for msg in &app.messages {
        let content = format_message_content(&msg.content, available_width);
        // Role prefix
        let (prefix, style) = match msg.role {
            MessageRole::User => ("You", Theme::user_message()),
            MessageRole::Assistant => ("Agent", Theme::assistant_message()),
            MessageRole::System => ("System", Theme::system_message()),
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{}: ", prefix), style),
        ]));

        // Message content - wrap text to fit viewport
        for line in content.lines() {
            if line.is_empty() {
                lines.push(Line::from("  "));
            } else {
                // Manually wrap long lines to prevent overflow
                let indent = "  ";
                let max_line_width = available_width.saturating_sub(indent.len());
                
                if line.chars().count() <= max_line_width {
                    // Line fits, add as-is
                    lines.push(Line::from(vec![
                        Span::raw(indent),
                        Span::styled(line.to_string(), Theme::text()),
                    ]));
                } else {
                    // Line is too long, wrap it
                    let mut remaining = line;
                    while !remaining.is_empty() {
                        if remaining.chars().count() <= max_line_width {
                            lines.push(Line::from(vec![
                                Span::raw(indent),
                                Span::styled(remaining.to_string(), Theme::text()),
                            ]));
                            break;
                        } else {
                            // Find a good breaking point (space, comma, etc.)
                            let mut break_byte = None;
                            let mut seen = 0usize;
                            for (idx, ch) in remaining.char_indices() {
                                if seen >= max_line_width {
                                    break;
                                }
                                if ch.is_whitespace() || ch == ',' || ch == '.' || ch == ';' {
                                    break_byte = Some(idx);
                                }
                                seen += 1;
                            }
                            let split_at = break_byte.unwrap_or_else(|| {
                                // Fallback: split at char boundary nearest max_line_width
                                let mut last_idx = 0usize;
                                let mut count = 0usize;
                                for (idx, _ch) in remaining.char_indices() {
                                    if count >= max_line_width {
                                        break;
                                    }
                                    last_idx = idx;
                                    count += 1;
                                }
                                if last_idx == 0 {
                                    remaining.len()
                                } else {
                                    last_idx
                                }
                            });

                            let (chunk, rest) = remaining.split_at(split_at);
                            lines.push(Line::from(vec![
                                Span::raw(indent),
                                Span::styled(chunk.to_string(), Theme::text()),
                            ]));
                            remaining = rest.trim_start();
                        }
                    }
                }
            }
        }

        lines.push(Line::from("")); // Spacing
    }

    // Show typing indicator if generating
    if matches!(app.pipeline_stage, PipelineStage::Generating) {
        lines.push(Line::from(vec![
            Span::styled("Agent: ", Theme::assistant_message()),
            Span::styled(Icons::CURSOR, Theme::active()),
        ]));
    }

    // Create paragraph with scroll
    let paragraph = Paragraph::new(lines)
        .scroll((app.scroll_offset, 0));

    frame.render_widget(paragraph, inner_area);
}

fn format_message_content(content: &str, available_width: usize) -> String {
    format_markdown_tables(content, available_width)
}

fn format_markdown_tables(content: &str, available_width: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let next = if i + 1 < lines.len() { lines[i + 1] } else { "" };
        if line.contains('|') && next.contains('|') && next.contains("---") {
            let header = line.trim().to_string();
            let mut rows: Vec<String> = Vec::new();
            let mut current: Option<String> = None;
            i += 2;
            while i < lines.len() {
                let l = lines[i];
                if l.trim().is_empty() {
                    break;
                }
                if l.trim_start().starts_with('|') {
                    if let Some(row) = current.take() {
                        rows.push(row);
                    }
                    current = Some(l.trim().to_string());
                } else if let Some(row) = current.as_mut() {
                    row.push(' ');
                    row.push_str(l.trim());
                }
                i += 1;
            }
            if let Some(row) = current.take() {
                rows.push(row);
            }
            if !rows.is_empty() {
                let header_cols = split_table_row(&header);
                let col_count = header_cols.len();
                if col_count <= 5 {
                    out.push(format_rows_as_table(&header_cols, &rows, available_width));
                } else {
                    out.push(format_rows_as_split_table(&header_cols, &rows, available_width));
                }
            }
            out.push(String::new());
        } else {
            out.push(line.to_string());
        }
        i += 1;
    }
    out.join("\n")
}

fn format_rows_as_table(header_cols: &[String], rows: &[String], available_width: usize) -> String {
    let col_count = header_cols.len();
    let sep_width = (col_count * 3) + 1; // "| " + " |" per col, plus leading "|"
    if available_width <= sep_width + col_count {
        return rows.join("\n");
    }

    let content_width = available_width.saturating_sub(sep_width);
    let weights = match col_count {
        1 => vec![1],
        2 => vec![3, 7],
        3 => vec![2, 3, 5],
        4 => vec![2, 3, 3, 4],
        5 => vec![2, 3, 3, 4, 4],
        _ => vec![1; col_count],
    };
    let weight_sum: usize = weights.iter().sum();
    let mut widths: Vec<usize> = weights
        .iter()
        .map(|w| (content_width * *w) / weight_sum)
        .collect();

    // Ensure minimum width for each column
    for w in widths.iter_mut() {
        if *w < 8 {
            *w = 8;
        }
    }

    // Adjust last column to fit exactly
    let used: usize = widths.iter().sum();
    if used != content_width {
        let last = widths.len() - 1;
        if used > content_width {
            let over = used - content_width;
            widths[last] = widths[last].saturating_sub(over);
        } else {
            widths[last] += content_width - used;
        }
    }

    let mut out = String::new();
    out.push_str(&format_table_row(header_cols, &widths));
    out.push('\n');
    out.push_str(&format_table_separator(&widths));
    out.push('\n');

    for (idx, row) in rows.iter().enumerate() {
        let cols = split_table_row(row);
        let mut wrapped_cols: Vec<Vec<String>> = Vec::new();
        for (i, width) in widths.iter().enumerate() {
            let value = cols.get(i).map(|s| s.as_str()).unwrap_or("");
            wrapped_cols.push(wrap_cell(value, *width));
        }
        let max_lines = wrapped_cols.iter().map(|c| c.len()).max().unwrap_or(1);
        for line_idx in 0..max_lines {
            let mut line = String::new();
            line.push('|');
            for (col_idx, width) in widths.iter().enumerate() {
                let cell_line = wrapped_cols[col_idx]
                    .get(line_idx)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                line.push(' ');
                line.push_str(&pad_cell(cell_line, *width));
                line.push(' ');
                line.push('|');
            }
            out.push_str(&line);
            out.push('\n');
        }
        if idx + 1 < rows.len() {
            out.push('\n');
        }
    }

    out.trim_end().to_string()
}

fn format_rows_as_split_table(header_cols: &[String], rows: &[String], available_width: usize) -> String {
    let col_count = header_cols.len();
    if col_count <= 5 {
        return format_rows_as_table(header_cols, rows, available_width);
    }

    let left_cols: Vec<usize> = (0..4.min(col_count)).collect();
    let mut right_cols: Vec<usize> = vec![0];
    for idx in 4..col_count {
        right_cols.push(idx);
    }

    let left_header = select_columns(header_cols, &left_cols);
    let right_header = select_columns(header_cols, &right_cols);

    let left_rows = rows
        .iter()
        .map(|r| select_columns(&split_table_row(r), &left_cols).join(" | "))
        .map(|r| format!("| {} |", r))
        .collect::<Vec<_>>();
    let right_rows = rows
        .iter()
        .map(|r| select_columns(&split_table_row(r), &right_cols).join(" | "))
        .map(|r| format!("| {} |", r))
        .collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format_rows_as_table(&left_header, &left_rows, available_width));
    out.push_str("\n\n");
    out.push_str(&format_rows_as_table(&right_header, &right_rows, available_width));
    out
}

fn format_rows_as_cards(rows: &[String]) -> String {
    let labels = [
        "Subtype(s)",
        "Location ",
        "Function ",
        "Partners ",
        "Disease  ",
        "Refs     ",
    ];
    let mut out = String::new();
    for (idx, row) in rows.iter().enumerate() {
        let cols = split_table_row(row);
        if cols.is_empty() {
            continue;
        }
        let title = strip_markdown_bold(cols.get(0).cloned().unwrap_or_default());
        out.push_str("[ ");
        out.push_str(title.trim());
        out.push_str(" ]\n");

        for (i, label) in labels.iter().enumerate() {
            let value = cols.get(i + 1).cloned().unwrap_or_default();
            let value = value.trim();
            if value.is_empty() {
                continue;
            }
            out.push_str(label);
            out.push_str(": ");
            out.push_str(value);
            out.push('\n');
        }

        if idx + 1 < rows.len() {
            out.push('\n');
        }
    }

    out
}

fn select_columns(cols: &[String], indices: &[usize]) -> Vec<String> {
    indices.iter().map(|i| cols.get(*i).cloned().unwrap_or_default()).collect()
}

fn split_table_row(row: &str) -> Vec<String> {
    let trimmed = row.trim();
    let trimmed = trimmed.trim_matches('|');
    trimmed
        .split('|')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn strip_markdown_bold(s: String) -> String {
    let trimmed = s.trim();
    if trimmed.starts_with("**") && trimmed.ends_with("**") && trimmed.len() >= 4 {
        trimmed[2..trimmed.len() - 2].to_string()
    } else {
        s
    }
}

fn format_table_row(cols: &[String], widths: &[usize]) -> String {
    let mut line = String::new();
    line.push('|');
    for (i, width) in widths.iter().enumerate() {
        let value = cols.get(i).map(|s| s.as_str()).unwrap_or("");
        line.push(' ');
        line.push_str(&pad_cell(value, *width));
        line.push(' ');
        line.push('|');
    }
    line
}

fn format_table_separator(widths: &[usize]) -> String {
    let mut line = String::new();
    line.push('|');
    for width in widths {
        line.push_str(&"-".repeat(width + 2));
        line.push('|');
    }
    line
}

fn pad_cell(value: &str, width: usize) -> String {
    let len = value.chars().count();
    if len >= width {
        value.chars().take(width).collect()
    } else {
        let mut s = String::with_capacity(width);
        s.push_str(value);
        s.push_str(&" ".repeat(width - len));
        s
    }
}

fn wrap_cell(value: &str, width: usize) -> Vec<String> {
    if value.is_empty() || width == 0 {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut remaining = value.trim();
    while !remaining.is_empty() {
        if remaining.chars().count() <= width {
            lines.push(remaining.to_string());
            break;
        }
        let mut break_byte = None;
        let mut seen = 0usize;
        for (idx, ch) in remaining.char_indices() {
            if seen >= width {
                break;
            }
            if ch.is_whitespace() || ch == ',' || ch == '.' || ch == ';' {
                break_byte = Some(idx);
            }
            seen += 1;
        }
        let split_at = break_byte.unwrap_or_else(|| {
            let mut last_idx = 0usize;
            let mut count = 0usize;
            for (idx, _ch) in remaining.char_indices() {
                if count >= width {
                    break;
                }
                last_idx = idx;
                count += 1;
            }
            if last_idx == 0 { remaining.len() } else { last_idx }
        });
        let (chunk, rest) = remaining.split_at(split_at);
        lines.push(chunk.trim().to_string());
        remaining = rest.trim_start();
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Render the input area
fn render_input(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.view == View::Chat;

    let block = Block::default()
        .title(" Input ")
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Theme::border_focused()
        } else {
            Theme::border()
        });

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Render textarea directly (widget() method is deprecated)
    frame.render_widget(&app.input, inner);
}

/// Render the status bar
fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let status = match &app.pipeline_stage {
        PipelineStage::Idle => Span::styled("Ready", Theme::text_secondary()),
        PipelineStage::Planning => Span::styled("Planning research...", Theme::active()),
        PipelineStage::Literature {
            task_index,
            total,
            current_task: _,
        } => Span::styled(
            format!("Literature search ({}/{})", task_index + 1, total),
            Theme::active(),
        ),
        PipelineStage::Generating => Span::styled("Generating response...", Theme::active()),
        PipelineStage::Complete => Span::styled("Complete", Theme::complete()),
        PipelineStage::Error(e) => Span::styled(format!("Error: {}", e), Theme::error()),
    };

    let shortcuts = vec![
        Span::styled(" [Enter]", Theme::shortcut_key()),
        Span::styled(" Send ", Theme::shortcut_desc()),
        Span::styled("[Ctrl+S]", Theme::shortcut_key()),
        Span::styled(" Settings ", Theme::shortcut_desc()),
        Span::styled("[Ctrl+Q]", Theme::shortcut_key()),
        Span::styled(" Quit ", Theme::shortcut_desc()),
        Span::styled("[F1]", Theme::shortcut_key()),
        Span::styled(" Help", Theme::shortcut_desc()),
    ];

    let tps_value = if matches!(app.pipeline_stage, PipelineStage::Generating) {
        app.stream_tps
    } else {
        app.last_stream_tps
    };

    let mut parts = Vec::new();
    parts.push(status);
    if tps_value > 0.0 {
        parts.push(Span::raw(" │ "));
        parts.push(Span::styled(format!("tok/s~ {:.1}", tps_value), Theme::text_dim()));
    }
    parts.push(Span::raw(" │ "));
    parts.extend(shortcuts);

    let line = Line::from(
        parts,
    );

    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}

/// Render the help modal
fn render_help(frame: &mut Frame) {
    let area = centered_rect(60, 60, frame.area());
    frame.render_widget(Clear, area);

    let help_lines = vec![
        Line::from(Span::styled("Keyboard Shortcuts", Theme::heading())),
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter        ", Theme::shortcut_key()),
            Span::styled("Send message / Confirm", Theme::text()),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+S       ", Theme::shortcut_key()),
            Span::styled("Open settings", Theme::text()),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+Q       ", Theme::shortcut_key()),
            Span::styled("Quit application", Theme::text()),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+C       ", Theme::shortcut_key()),
            Span::styled("Force quit", Theme::text()),
        ]),
        Line::from(vec![
            Span::styled("↑/↓          ", Theme::shortcut_key()),
            Span::styled("Scroll messages", Theme::text()),
        ]),
        Line::from(vec![
            Span::styled("PageUp/Down  ", Theme::shortcut_key()),
            Span::styled("Scroll page", Theme::text()),
        ]),
        Line::from(vec![
            Span::styled("Tab          ", Theme::shortcut_key()),
            Span::styled("Next field (in settings)", Theme::text()),
        ]),
        Line::from(vec![
            Span::styled("Esc          ", Theme::shortcut_key()),
            Span::styled("Close modal / Cancel", Theme::text()),
        ]),
        Line::from(vec![
            Span::styled("F1 / ?       ", Theme::shortcut_key()),
            Span::styled("Show this help", Theme::text()),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Theme::text_dim(),
        )),
    ];

    let paragraph = Paragraph::new(help_lines).block(
        Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(Theme::border_focused()),
    );

    frame.render_widget(paragraph, area);
}

/// Helper to create a centered rect
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
