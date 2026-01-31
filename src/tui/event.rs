//! Event Handling
//!
//! Handles keyboard, mouse, and timer events for the TUI.

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use futures::{FutureExt, StreamExt};
use std::time::Duration;
use tokio::sync::mpsc;

/// Actions that can be performed in the application
#[derive(Debug, Clone)]
pub enum AppAction {
    /// Quit the application (with confirmation if needed)
    Quit,
    /// Force quit without confirmation
    ForceQuit,
    /// Submit current input (Enter key)
    Submit,
    /// Toggle settings view
    ToggleSettings,
    /// Toggle help view
    ToggleHelp,
    /// Escape - close modals, cancel
    Escape,
    /// Scroll up one line
    ScrollUp,
    /// Scroll down one line
    ScrollDown,
    /// Scroll up one page
    ScrollPageUp,
    /// Scroll down one page
    ScrollPageDown,
    /// Move to next field (Tab)
    NextField,
    /// Move to previous field (Shift+Tab)
    PrevField,
    /// Start editing current field
    EditField,
    /// Delete character
    DeleteKey,
    /// Regular input character
    Input(KeyEvent),
    /// Timer tick for animations
    Tick,
}

/// Event handler for the TUI
pub struct EventHandler {
    rx: mpsc::Receiver<AppAction>,
    _tx: mpsc::Sender<AppAction>,
}

impl EventHandler {
    /// Create a new event handler with specified tick rate
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let tx_clone = tx.clone();

        // Spawn event polling task
        tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_rate);

            loop {
                let tick = tick_interval.tick();
                let crossterm_event = reader.next().fuse();

                tokio::select! {
                    _ = tick => {
                        if tx_clone.send(AppAction::Tick).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(evt)) = crossterm_event => {
                        if let Some(action) = Self::map_event(evt) {
                            if tx_clone.send(action).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });

        Self { rx, _tx: tx }
    }

    /// Try to get the next action without blocking
    pub async fn try_next(&mut self) -> Option<AppAction> {
        self.rx.try_recv().ok()
    }

    /// Wait for the next action
    pub async fn next(&mut self) -> Option<AppAction> {
        self.rx.recv().await
    }

    /// Map a crossterm event to an app action
    fn map_event(event: Event) -> Option<AppAction> {
        match event {
            Event::Key(key) => Self::map_key_event(key),
            Event::Mouse(_) => None, // Could handle mouse events here
            Event::Resize(_, _) => None, // Terminal handles resize
            _ => None,
        }
    }

    /// Map a key event to an app action
    fn map_key_event(key: KeyEvent) -> Option<AppAction> {
        // Handle key with modifiers first
        match (key.modifiers, key.code) {
            // Quit shortcuts
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => Some(AppAction::ForceQuit),
            (KeyModifiers::CONTROL, KeyCode::Char('q')) => Some(AppAction::Quit),

            // View toggles
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => Some(AppAction::ToggleSettings),
            (KeyModifiers::CONTROL, KeyCode::Char('h')) => Some(AppAction::ToggleHelp),

            // Navigation with modifiers
            (KeyModifiers::SHIFT, KeyCode::BackTab) => Some(AppAction::PrevField),

            // No modifiers
            (KeyModifiers::NONE, code) | (KeyModifiers::SHIFT, code) => match code {
                // Escape
                KeyCode::Esc => Some(AppAction::Escape),

                // Submit
                KeyCode::Enter => Some(AppAction::Submit),

                // Help
                KeyCode::F(1) => Some(AppAction::ToggleHelp),
                KeyCode::Char('?') if key.modifiers == KeyModifiers::NONE => {
                    // Only ? without shift triggers help in chat view
                    // In input mode, pass through
                    Some(AppAction::Input(key))
                }

                // Scrolling
                KeyCode::Up => Some(AppAction::ScrollUp),
                KeyCode::Down => Some(AppAction::ScrollDown),
                KeyCode::PageUp => Some(AppAction::ScrollPageUp),
                KeyCode::PageDown => Some(AppAction::ScrollPageDown),
                KeyCode::Home => Some(AppAction::ScrollPageUp), // Jump to top
                KeyCode::End => Some(AppAction::ScrollPageDown), // Jump to bottom

                // Tab navigation
                KeyCode::Tab => Some(AppAction::NextField),

                // Editing
                KeyCode::Backspace => Some(AppAction::DeleteKey),

                // Edit field trigger (Enter or specific key in settings)
                KeyCode::Char('e') if key.modifiers == KeyModifiers::NONE => {
                    Some(AppAction::EditField)
                }

                // All other characters are input
                _ => Some(AppAction::Input(key)),
            },

            // Pass through other key combinations as input
            _ => Some(AppAction::Input(key)),
        }
    }
}
