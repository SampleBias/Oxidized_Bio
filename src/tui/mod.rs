//! Terminal User Interface Module
//!
//! Provides a rich terminal interface for the Oxidized Bio research agent.
//! Built with Ratatui for high-performance terminal rendering.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  🧬 Oxidized Bio Research Agent                    [Settings ⚙] │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  ┌─ Research Progress ─────────────────────────────────────┐   │
//! │  │ ● Planning → ○ Literature → ○ Generating → ○ Done       │   │
//! │  └─────────────────────────────────────────────────────────┘   │
//! │  ┌─ Response ──────────────────────────────────────────────┐   │
//! │  │  [Scrollable message history with markdown support]      │   │
//! │  └─────────────────────────────────────────────────────────┘   │
//! │  ┌─ Input ─────────────────────────────────────────────────┐   │
//! │  │ > Type your research question here...                    │   │
//! │  └─────────────────────────────────────────────────────────┘   │
//! │  [Enter] Send | [Ctrl+S] Settings | [Ctrl+Q] Quit | [?] Help   │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

pub mod app;
pub mod event;
pub mod theme;
pub mod ui;
pub mod widgets;

pub use app::{App, AppEvent, PipelineStage, View};
pub use event::{AppAction, EventHandler};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};
use tracing::{error, info};

/// Type alias for our terminal backend
pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Initialize the terminal for TUI mode
pub fn init_terminal() -> anyhow::Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal to its original state
pub fn restore_terminal(terminal: &mut Tui) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Run the TUI application
pub async fn run(config: crate::config::Config) -> anyhow::Result<()> {
    info!("Starting TUI mode");

    // Initialize terminal
    let mut terminal = init_terminal()?;

    // Create application state
    let mut app = App::new(config);

    // Create event handler
    let mut events = EventHandler::new(std::time::Duration::from_millis(100));

    // Main loop
    let result = run_app(&mut terminal, &mut app, &mut events).await;

    // Restore terminal
    if let Err(e) = restore_terminal(&mut terminal) {
        error!("Failed to restore terminal: {}", e);
    }

    result
}

/// Main application loop
async fn run_app(
    terminal: &mut Tui,
    app: &mut App,
    events: &mut EventHandler,
) -> anyhow::Result<()> {
    loop {
        // Update scroll bounds before drawing
        app.calculate_scroll_bounds(terminal.size()?.height);
        
        // Draw UI
        terminal.draw(|frame| ui::render(frame, app))?;

        // Handle async events from research pipeline
        app.poll_events();

        // Handle user input
        if let Some(action) = events.try_next().await {
            match action {
                AppAction::Quit => {
                    if app.confirm_quit() {
                        break;
                    }
                }
                AppAction::ForceQuit => break,
                _ => app.handle_action(action).await,
            }
        }

        // Small yield to prevent busy loop
        tokio::task::yield_now().await;
    }

    info!("TUI exited normally");
    Ok(())
}
