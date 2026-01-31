//! Progress Widget
//!
//! Displays the research pipeline progress.

use crate::tui::app::PipelineStage;
use crate::tui::theme::{Icons, Theme};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the progress indicator
pub fn render_progress(
    frame: &mut Frame,
    area: Rect,
    stage: &PipelineStage,
    objective: &Option<String>,
) {
    let block = Block::default()
        .title(" Research Progress ")
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = Vec::new();

    // Objective line
    if let Some(obj) = objective {
        lines.push(Line::from(vec![
            Span::styled("Objective: ", Theme::text_secondary()),
            Span::styled(truncate_string(obj, inner.width as usize - 12), Theme::text()),
        ]));
    } else {
        lines.push(Line::from(vec![Span::styled(
            "Waiting for input...",
            Theme::text_dim(),
        )]));
    }

    // Progress indicator line
    let progress_spans = build_progress_line(stage);
    lines.push(Line::from(progress_spans));

    // Current task detail (if in literature stage)
    if let PipelineStage::Literature { current_task, .. } = stage {
        lines.push(Line::from(vec![
            Span::styled("  Task: ", Theme::text_dim()),
            Span::styled(
                truncate_string(current_task, inner.width as usize - 10),
                Theme::text_secondary(),
            ),
        ]));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Build the progress line with stage indicators
fn build_progress_line(stage: &PipelineStage) -> Vec<Span<'static>> {
    let stages = [
        ("Planning", StageState::from_planning(stage)),
        ("Literature", StageState::from_literature(stage)),
        ("Generating", StageState::from_generating(stage)),
        ("Done", StageState::from_done(stage)),
    ];

    let mut spans = Vec::new();

    for (i, (name, state)) in stages.iter().enumerate() {
        let (icon, style) = match state {
            StageState::Complete => (Icons::COMPLETE, Theme::complete()),
            StageState::Active => (Icons::ACTIVE, Theme::active()),
            StageState::Pending => (Icons::PENDING, Theme::pending()),
            StageState::Error => (Icons::ERROR, Theme::error()),
        };

        spans.push(Span::styled(format!("{} ", icon), style));
        spans.push(Span::styled(name.to_string(), style));

        // Add arrow between stages (not after last)
        if i < stages.len() - 1 {
            spans.push(Span::styled(format!(" {} ", Icons::ARROW), Theme::text_dim()));
        }
    }

    spans
}

/// State of a pipeline stage
#[derive(Debug, Clone, Copy, PartialEq)]
enum StageState {
    Pending,
    Active,
    Complete,
    Error,
}

impl StageState {
    fn from_planning(stage: &PipelineStage) -> Self {
        match stage {
            PipelineStage::Idle => StageState::Pending,
            PipelineStage::Planning => StageState::Active,
            PipelineStage::Error(_) => StageState::Error,
            _ => StageState::Complete,
        }
    }

    fn from_literature(stage: &PipelineStage) -> Self {
        match stage {
            PipelineStage::Idle | PipelineStage::Planning => StageState::Pending,
            PipelineStage::Literature { .. } => StageState::Active,
            PipelineStage::Error(_) => StageState::Error,
            _ => StageState::Complete,
        }
    }

    fn from_generating(stage: &PipelineStage) -> Self {
        match stage {
            PipelineStage::Idle | PipelineStage::Planning | PipelineStage::Literature { .. } => {
                StageState::Pending
            }
            PipelineStage::Generating => StageState::Active,
            PipelineStage::Error(_) => StageState::Error,
            PipelineStage::Complete => StageState::Complete,
        }
    }

    fn from_done(stage: &PipelineStage) -> Self {
        match stage {
            PipelineStage::Complete => StageState::Complete,
            PipelineStage::Error(_) => StageState::Error,
            _ => StageState::Pending,
        }
    }
}

/// Truncate a string to fit within a given width
fn truncate_string(s: &str, max_width: usize) -> String {
    if s.len() <= max_width {
        s.to_string()
    } else if max_width > 3 {
        format!("{}...", &s[..max_width - 3])
    } else {
        s[..max_width].to_string()
    }
}
