//! Settings Widget
//!
//! Modal dialog for configuring API keys and preferences.

use crate::tui::app::App;
use crate::tui::theme::{Icons, Theme};
use crate::tui::ui::centered_rect;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Render the settings modal
pub fn render_settings(frame: &mut Frame, app: &App) {
    // Create centered modal
    let area = centered_rect(70, 70, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Settings ")
        .borders(Borders::ALL)
        .border_style(Theme::border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Instructions
            Constraint::Min(10),    // Provider list
            Constraint::Length(2),  // Footer
        ])
        .split(inner);

    render_instructions(frame, chunks[0]);
    render_provider_list(frame, chunks[1], app);
    render_footer(frame, chunks[2], app);
}

/// Render instructions
fn render_instructions(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(Span::styled(
            "Configure your API keys (LLM + Search). Keys are encrypted locally.",
            Theme::text(),
        )),
        Line::from(vec![
            Span::styled("[Tab]", Theme::shortcut_key()),
            Span::styled(" Next ", Theme::shortcut_desc()),
            Span::styled("[e]", Theme::shortcut_key()),
            Span::styled(" Edit ", Theme::shortcut_desc()),
            Span::styled("[Enter]", Theme::shortcut_key()),
            Span::styled(" Save ", Theme::shortcut_desc()),
            Span::styled("[Esc]", Theme::shortcut_key()),
            Span::styled(" Close", Theme::shortcut_desc()),
        ]),
    ];

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Render the provider list
fn render_provider_list(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines = Vec::new();

    for (i, provider) in app.providers.iter().enumerate() {
        let is_selected = i == app.settings_field_index;

        // Provider row
        let prefix = if is_selected {
            Icons::SELECTED
        } else {
            " "
        };

        let status = if provider.has_key {
            Span::styled(
                format!(" {} Configured", Icons::COMPLETE),
                Theme::success(),
            )
        } else {
            Span::styled(format!(" {} Not set", Icons::PENDING), Theme::text_dim())
        };

        let name_style = if is_selected {
            Theme::selected()
        } else {
            Theme::text()
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{} ", prefix), if is_selected { Theme::selected() } else { Theme::text_dim() }),
            Span::styled(format!("{:<15}", provider.name), name_style),
            status,
        ]));

        // Show key hint or input field if selected
        if is_selected {
            if app.settings_show_input {
                // Show input field
                let input_display = if app.settings_input.is_empty() {
                    "Enter API key...".to_string()
                } else {
                    Icons::DOT.repeat(app.settings_input.len())
                };

                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(format!("[{}]", input_display), Theme::warning()),
                    Span::styled(" ", Theme::text()),
                    Span::styled(Icons::CURSOR, Theme::active()),
                ]));
            } else if let Some(hint) = &provider.key_hint {
                // Show masked key
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(format!("Key: {}", hint), Theme::text_dim()),
                    Span::styled("  [e] to change", Theme::text_dim()),
                ]));
            } else {
                // Show prompt to add key
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled("Press [e] to enter API key", Theme::text_dim()),
                ]));
            }
        }

        lines.push(Line::from("")); // Spacing
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Render the footer
fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let current_provider = &app.providers[app.settings_field_index];
    
    let help_text = if app.settings_show_input {
        "Type your API key, then press Enter to save"
    } else if current_provider.has_key {
        "Press [e] to change key, or Tab to select another provider"
    } else {
        "Press [e] to enter an API key"
    };

    let line = Line::from(Span::styled(help_text, Theme::text_secondary()));
    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}
