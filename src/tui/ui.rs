//! UI Rendering
//!
//! Main UI layout and rendering logic for the TUI.

use crate::tui::app::{App, MessageRole, PipelineStage, View};
use crate::tui::theme::{Icons, Theme};
use crate::tui::widgets;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

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

    render_header(frame, chunks[0]);
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

/// Render the header
fn render_header(frame: &mut Frame, area: Rect) {
    let title_text = vec![Line::from(vec![
        Span::raw("🧬 "),
        Span::styled("Oxidized Bio", Theme::title()),
        Span::styled(" Research Agent", Theme::text_secondary()),
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
        for line in msg.content.lines() {
            if line.is_empty() {
                lines.push(Line::from("  "));
            } else {
                // Manually wrap long lines to prevent overflow
                let indent = "  ";
                let max_line_width = available_width.saturating_sub(indent.len());
                
                if line.len() <= max_line_width {
                    // Line fits, add as-is
                    lines.push(Line::from(vec![
                        Span::raw(indent),
                        Span::styled(line, Theme::text()),
                    ]));
                } else {
                    // Line is too long, wrap it
                    let mut remaining = line;
                    while !remaining.is_empty() {
                        if remaining.len() <= max_line_width {
                            lines.push(Line::from(vec![
                                Span::raw(indent),
                                Span::styled(remaining, Theme::text()),
                            ]));
                            break;
                        } else {
                            // Find a good breaking point (space, comma, etc.)
                            let break_point = remaining[..max_line_width]
                                .rfind(|c: char| c.is_whitespace() || c == ',' || c == '.' || c == ';')
                                .unwrap_or(max_line_width);
                            
                            let (chunk, rest) = remaining.split_at(break_point);
                            lines.push(Line::from(vec![
                                Span::raw(indent),
                                Span::styled(chunk, Theme::text()),
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

    let line = Line::from(
        std::iter::once(status)
            .chain(std::iter::once(Span::raw(" │ ")))
            .chain(shortcuts)
            .collect::<Vec<_>>(),
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
