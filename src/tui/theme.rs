//! Theme and Styling
//!
//! Defines colors and styles for the TUI interface.

use ratatui::style::{Color, Modifier, Style};

/// Application theme
pub struct Theme;

impl Theme {
    // === Primary Colors ===

    /// Primary accent color (cyan/teal)
    pub const ACCENT: Color = Color::Rgb(0, 212, 255);

    /// Secondary accent (green)
    pub const SUCCESS: Color = Color::Rgb(34, 197, 94);

    /// Warning color (yellow/amber)
    pub const WARNING: Color = Color::Rgb(251, 191, 36);

    /// Error color (red)
    pub const ERROR: Color = Color::Rgb(239, 68, 68);

    // === Text Colors ===

    /// Primary text color
    pub const TEXT_PRIMARY: Color = Color::Rgb(229, 229, 229);

    /// Secondary text color (muted)
    pub const TEXT_SECONDARY: Color = Color::Rgb(161, 161, 161);

    /// Dimmed text
    pub const TEXT_DIM: Color = Color::Rgb(82, 82, 82);

    // === Background Colors ===

    /// Primary background
    pub const BG_PRIMARY: Color = Color::Rgb(10, 10, 10);

    /// Secondary background (slightly lighter)
    pub const BG_SECONDARY: Color = Color::Rgb(26, 26, 26);

    /// Highlighted/selected background
    pub const BG_HIGHLIGHT: Color = Color::Rgb(38, 38, 38);

    // === Border Colors ===

    /// Default border color
    pub const BORDER: Color = Color::Rgb(51, 51, 51);

    /// Focused border color
    pub const BORDER_FOCUSED: Color = Color::Rgb(59, 130, 246);

    // === Role Colors ===

    /// User message color
    pub const USER: Color = Color::Rgb(34, 197, 94);

    /// Assistant message color
    pub const ASSISTANT: Color = Color::Rgb(0, 212, 255);

    /// System message color
    pub const SYSTEM: Color = Color::Rgb(251, 191, 36);

    // === Styles ===

    /// Default text style
    pub fn text() -> Style {
        Style::default().fg(Self::TEXT_PRIMARY)
    }

    /// Secondary/muted text style
    pub fn text_secondary() -> Style {
        Style::default().fg(Self::TEXT_SECONDARY)
    }

    /// Dimmed text style
    pub fn text_dim() -> Style {
        Style::default().fg(Self::TEXT_DIM)
    }

    /// Title style
    pub fn title() -> Style {
        Style::default()
            .fg(Self::ACCENT)
            .add_modifier(Modifier::BOLD)
    }

    /// Heading style
    pub fn heading() -> Style {
        Style::default()
            .fg(Self::TEXT_PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    /// Success style
    pub fn success() -> Style {
        Style::default().fg(Self::SUCCESS)
    }

    /// Warning style
    pub fn warning() -> Style {
        Style::default().fg(Self::WARNING)
    }

    /// Error style
    pub fn error() -> Style {
        Style::default().fg(Self::ERROR)
    }

    /// Default border style
    pub fn border() -> Style {
        Style::default().fg(Self::BORDER)
    }

    /// Focused border style
    pub fn border_focused() -> Style {
        Style::default().fg(Self::BORDER_FOCUSED)
    }

    /// Selected item style
    pub fn selected() -> Style {
        Style::default()
            .fg(Self::ACCENT)
            .add_modifier(Modifier::BOLD)
    }

    /// User message style
    pub fn user_message() -> Style {
        Style::default()
            .fg(Self::USER)
            .add_modifier(Modifier::BOLD)
    }

    /// Assistant message style
    pub fn assistant_message() -> Style {
        Style::default()
            .fg(Self::ASSISTANT)
            .add_modifier(Modifier::BOLD)
    }

    /// System message style
    pub fn system_message() -> Style {
        Style::default()
            .fg(Self::SYSTEM)
            .add_modifier(Modifier::BOLD)
    }

    /// Keyboard shortcut style
    pub fn shortcut_key() -> Style {
        Style::default()
            .fg(Self::ACCENT)
            .add_modifier(Modifier::BOLD)
    }

    /// Shortcut description style
    pub fn shortcut_desc() -> Style {
        Style::default().fg(Self::TEXT_SECONDARY)
    }

    /// Active/in-progress indicator
    pub fn active() -> Style {
        Style::default()
            .fg(Self::WARNING)
            .add_modifier(Modifier::BOLD)
    }

    /// Complete indicator
    pub fn complete() -> Style {
        Style::default().fg(Self::SUCCESS)
    }

    /// Pending indicator
    pub fn pending() -> Style {
        Style::default().fg(Self::TEXT_DIM)
    }

    /// Input placeholder style
    pub fn placeholder() -> Style {
        Style::default().fg(Self::TEXT_DIM)
    }

    /// Badge style (for labels like "Configured", "Default")
    pub fn badge_success() -> Style {
        Style::default()
            .fg(Color::Black)
            .bg(Self::SUCCESS)
            .add_modifier(Modifier::BOLD)
    }

    /// Badge style for primary/default
    pub fn badge_primary() -> Style {
        Style::default()
            .fg(Color::Black)
            .bg(Self::ACCENT)
            .add_modifier(Modifier::BOLD)
    }
}

/// Progress stage icons
pub struct Icons;

impl Icons {
    pub const COMPLETE: &'static str = "✓";
    pub const ACTIVE: &'static str = "●";
    pub const PENDING: &'static str = "○";
    pub const ERROR: &'static str = "✗";
    pub const ARROW: &'static str = "→";
    pub const CURSOR: &'static str = "▌";
    pub const SELECTED: &'static str = "▶";
    pub const DOT: &'static str = "•";
}
