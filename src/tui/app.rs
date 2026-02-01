//! Application State
//!
//! Contains the main application state and logic for the TUI.

use crate::agents::{self, LiteratureResult, PlanningResult};
use crate::config::Config;
use crate::settings::{SettingsStorage, UserSettings};
use crate::tui::event::AppAction;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use tui_textarea::TextArea;

/// Research pipeline stage
#[derive(Debug, Clone, PartialEq)]
pub enum PipelineStage {
    /// Idle, waiting for input
    Idle,
    /// Planning research tasks
    Planning,
    /// Executing literature search
    Literature {
        task_index: usize,
        total: usize,
        current_task: String,
    },
    /// Generating response
    Generating,
    /// Research complete
    Complete,
    /// Error occurred
    Error(String),
}

impl Default for PipelineStage {
    fn default() -> Self {
        Self::Idle
    }
}

/// A chat message
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Message role
#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Current view/screen
#[derive(Debug, Clone, PartialEq, Default)]
pub enum View {
    #[default]
    Chat,
    Settings,
    Help,
}

/// Events from async research pipeline
#[derive(Debug)]
pub enum AppEvent {
    /// Pipeline stage changed
    StageChanged(PipelineStage),
    /// Current objective updated
    ObjectiveUpdated(String),
    /// Response chunk received (for streaming)
    ResponseChunk(String),
    /// Response complete
    ResponseComplete(String),
    /// Error occurred
    Error(String),
}

/// Provider configuration for settings view
#[derive(Debug, Clone)]
pub struct ProviderField {
    pub id: &'static str,
    pub name: &'static str,
    pub has_key: bool,
    pub key_hint: Option<String>,
}

/// Main application state
pub struct App {
    // Configuration
    pub config: Config,

    // UI State
    pub view: View,
    pub should_quit: bool,

    // Chat State
    pub messages: Vec<ChatMessage>,
    pub input: TextArea<'static>,
    pub scroll_offset: u16,
    pub max_scroll: u16,

    // Research State
    pub pipeline_stage: PipelineStage,
    pub current_objective: Option<String>,

    // Settings State
    pub settings: UserSettings,
    pub settings_storage: SettingsStorage,
    pub settings_field_index: usize,
    pub settings_input: String,
    pub settings_show_input: bool,
    pub providers: Vec<ProviderField>,

    // Async communication
    event_rx: Option<mpsc::Receiver<AppEvent>>,
    event_tx: Option<mpsc::Sender<AppEvent>>,
}

impl App {
    /// Create a new application instance
    pub fn new(config: Config) -> Self {
        // Initialize text input
        let mut input = TextArea::default();
        input.set_cursor_line_style(ratatui::style::Style::default());
        input.set_placeholder_text("Type your research question here...");

        // Load settings
        let settings_storage = SettingsStorage::new();
        let settings = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(settings_storage.load())
                .unwrap_or_default()
        });

        // Build provider list
        let providers = Self::build_provider_list(&settings);

        // Create event channel
        let (tx, rx) = mpsc::channel(100);

        // Add welcome message
        let messages = vec![ChatMessage {
            role: MessageRole::System,
            content: "Welcome to Oxidized Bio Research Agent!\n\n\
                     Type a research question below to get started.\n\
                     Press Ctrl+S to configure your API keys in Settings."
                .to_string(),
            timestamp: Utc::now(),
        }];

        let mut app = Self {
            config,
            view: View::Chat,
            should_quit: false,
            messages,
            input,
            scroll_offset: 0,
            max_scroll: 0,
            pipeline_stage: PipelineStage::Idle,
            current_objective: None,
            settings,
            settings_storage,
            settings_field_index: 0,
            settings_input: String::new(),
            settings_show_input: false,
            providers,
            event_rx: Some(rx),
            event_tx: Some(tx),
        };

        app.update_config_from_settings();
        app
    }

    /// Build the provider list from settings
    fn build_provider_list(settings: &UserSettings) -> Vec<ProviderField> {
        vec![
            ProviderField {
                id: "openai",
                name: "OpenAI",
                has_key: settings.openai.api_key.is_some(),
                key_hint: settings
                    .openai
                    .api_key
                    .as_ref()
                    .map(|k| format!("••••{}", &k[k.len().saturating_sub(4)..])),
            },
            ProviderField {
                id: "anthropic",
                name: "Anthropic",
                has_key: settings.anthropic.api_key.is_some(),
                key_hint: settings
                    .anthropic
                    .api_key
                    .as_ref()
                    .map(|k| format!("••••{}", &k[k.len().saturating_sub(4)..])),
            },
            ProviderField {
                id: "google",
                name: "Google AI",
                has_key: settings.google.api_key.is_some(),
                key_hint: settings
                    .google
                    .api_key
                    .as_ref()
                    .map(|k| format!("••••{}", &k[k.len().saturating_sub(4)..])),
            },
            ProviderField {
                id: "openrouter",
                name: "OpenRouter",
                has_key: settings.openrouter.api_key.is_some(),
                key_hint: settings
                    .openrouter
                    .api_key
                    .as_ref()
                    .map(|k| format!("••••{}", &k[k.len().saturating_sub(4)..])),
            },
        ]
    }

    /// Refresh provider list from current settings
    pub fn refresh_providers(&mut self) {
        self.providers = Self::build_provider_list(&self.settings);
    }

    /// Check if we should confirm quit
    pub fn confirm_quit(&self) -> bool {
        // If research is in progress, might want to confirm
        // For now, just quit
        true
    }

    /// Poll for async events
    pub fn poll_events(&mut self) {
        // Collect events first to avoid borrow checker issues
        let events: Vec<AppEvent> = {
            if let Some(ref mut rx) = self.event_rx {
                let mut collected = Vec::new();
                while let Ok(event) = rx.try_recv() {
                    collected.push(event);
                }
                collected
            } else {
                Vec::new()
            }
        };

        // Now handle collected events
        for event in events {
            self.handle_event(event);
        }
    }

    /// Handle an async event
    fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::StageChanged(stage) => {
                self.pipeline_stage = stage;
            }
            AppEvent::ObjectiveUpdated(objective) => {
                self.current_objective = Some(objective);
            }
            AppEvent::ResponseChunk(chunk) => {
                // Append to last assistant message if streaming
                if let Some(last) = self.messages.last_mut() {
                    if last.role == MessageRole::Assistant {
                        last.content.push_str(&chunk);
                    }
                }
            }
            AppEvent::ResponseComplete(response) => {
                self.messages.push(ChatMessage {
                    role: MessageRole::Assistant,
                    content: response,
                    timestamp: Utc::now(),
                });
                self.pipeline_stage = PipelineStage::Complete;
                self.scroll_to_bottom();
            }
            AppEvent::Error(error) => {
                self.pipeline_stage = PipelineStage::Error(error.clone());
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: format!("Error: {}", error),
                    timestamp: Utc::now(),
                });
            }
        }
    }

    /// Handle a user action
    pub async fn handle_action(&mut self, action: AppAction) {
        match action {
            AppAction::Quit | AppAction::ForceQuit => {
                self.should_quit = true;
            }
            AppAction::Submit => {
                if self.view == View::Settings {
                    self.save_current_setting().await;
                } else {
                    self.submit_message().await;
                }
            }
            AppAction::ToggleSettings => {
                self.view = if self.view == View::Settings {
                    View::Chat
                } else {
                    View::Settings
                };
            }
            AppAction::ToggleHelp => {
                self.view = if self.view == View::Help {
                    View::Chat
                } else {
                    View::Help
                };
            }
            AppAction::Escape => {
                if self.view != View::Chat {
                    self.view = View::Chat;
                    self.settings_show_input = false;
                    self.settings_input.clear();
                }
            }
            AppAction::ScrollUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
            AppAction::ScrollDown => {
                if self.scroll_offset < self.max_scroll {
                    self.scroll_offset += 1;
                }
            }
            AppAction::ScrollPageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
            }
            AppAction::ScrollPageDown => {
                self.scroll_offset = (self.scroll_offset + 10).min(self.max_scroll);
            }
            AppAction::NextField => {
                if self.view == View::Settings {
                    self.settings_field_index =
                        (self.settings_field_index + 1) % self.providers.len();
                    self.settings_show_input = false;
                    self.settings_input.clear();
                }
            }
            AppAction::PrevField => {
                if self.view == View::Settings {
                    self.settings_field_index = if self.settings_field_index == 0 {
                        self.providers.len() - 1
                    } else {
                        self.settings_field_index - 1
                    };
                    self.settings_show_input = false;
                    self.settings_input.clear();
                }
            }
            AppAction::Input(key_event) => {
                self.handle_input(key_event);
            }
            AppAction::Tick => {
                // Animation tick - could update spinner, etc.
            }
        }
    }

    /// Handle keyboard input
    fn handle_input(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        if self.view == View::Settings {
            if self.settings_show_input {
                // Settings input mode
                match key.code {
                    KeyCode::Char(c) => {
                        self.settings_input.push(c);
                    }
                    KeyCode::Backspace => {
                        self.settings_input.pop();
                    }
                    _ => {}
                }
            } else if key.modifiers == crossterm::event::KeyModifiers::NONE
                && key.code == KeyCode::Char('e')
            {
                // Enter edit mode for the selected provider
                self.settings_show_input = true;
            }
        } else if self.view == View::Chat {
            // Chat input mode - delegate to textarea
            self.input.input(key);
        }
    }

    /// Submit the current message
    async fn submit_message(&mut self) {
        let content: String = self.input.lines().join("\n");
        let content = content.trim().to_string();

        if content.is_empty() {
            return;
        }

        // Clear input
        self.input = TextArea::default();
        self.input.set_placeholder_text("Type your research question here...");

        // Add user message
        self.messages.push(ChatMessage {
            role: MessageRole::User,
            content: content.clone(),
            timestamp: Utc::now(),
        });

        // Check if any API key is configured
        let has_api_key = self.settings.openai.api_key.is_some()
            || self.settings.anthropic.api_key.is_some()
            || self.settings.google.api_key.is_some()
            || self.settings.openrouter.api_key.is_some();

        if !has_api_key {
            self.messages.push(ChatMessage {
                role: MessageRole::System,
                content: "No API key configured. Press Ctrl+S to open Settings and add an API key."
                    .to_string(),
                timestamp: Utc::now(),
            });
            return;
        }

        // Start research pipeline
        self.pipeline_stage = PipelineStage::Planning;
        self.current_objective = None;

        // Get event sender
        let tx = self.event_tx.clone().unwrap();
        let config = self.config.clone();

        // Spawn async research task
        tokio::spawn(async move {
            Self::run_research_pipeline(content, config, tx).await;
        });

        self.scroll_to_bottom();
    }

    /// Run the research pipeline in background
    async fn run_research_pipeline(message: String, config: Config, tx: mpsc::Sender<AppEvent>) {
        // Planning stage
        tx.send(AppEvent::StageChanged(PipelineStage::Planning))
            .await
            .ok();

        let planning_result = agents::PlanningAgent::generate_plan(&message, None, &config).await;

        match planning_result {
            Ok(plan) => {
                tx.send(AppEvent::ObjectiveUpdated(plan.current_objective.clone()))
                    .await
                    .ok();

                // Literature search stage
                let mut literature_results = Vec::new();
                let total_tasks = plan.plan.len();

                for (i, task) in plan.plan.iter().enumerate() {
                    tx.send(AppEvent::StageChanged(PipelineStage::Literature {
                        task_index: i,
                        total: total_tasks,
                        current_task: task.objective.clone(),
                    }))
                    .await
                    .ok();

                    match agents::LiteratureAgent::execute_task(task, &config).await {
                        Ok(result) => literature_results.push(result),
                        Err(e) => {
                            warn!("Literature task failed: {}", e);
                        }
                    }
                }

                // Generating stage
                tx.send(AppEvent::StageChanged(PipelineStage::Generating))
                    .await
                    .ok();

                let reply_mode = agents::ReplyAgent::classify_mode(&message);
                let response = agents::ReplyAgent::generate_response(
                    &message,
                    Some(&plan),
                    &literature_results,
                    reply_mode,
                    &config,
                )
                .await;

                match response {
                    Ok(text) => {
                        tx.send(AppEvent::ResponseComplete(text)).await.ok();
                    }
                    Err(e) => {
                        tx.send(AppEvent::Error(e.to_string())).await.ok();
                    }
                }
            }
            Err(e) => {
                tx.send(AppEvent::Error(e.to_string())).await.ok();
            }
        }
    }

    /// Save the current setting
    async fn save_current_setting(&mut self) {
        if !self.settings_show_input || self.settings_input.is_empty() {
            return;
        }

        let provider_id = self.providers[self.settings_field_index].id;
        let key = std::mem::take(&mut self.settings_input);

        // Update settings (only one provider key at a time)
        if crate::settings::Provider::from_id(provider_id).is_none() {
            return;
        }
        self.settings.set_single_provider_key(provider_id, key);

        // Save to storage
        if let Err(e) = self.settings_storage.save(&self.settings).await {
            error!("Failed to save settings: {}", e);
        } else {
            info!("Settings saved for provider: {}", provider_id);
        }

        // Refresh provider list
        self.refresh_providers();
        self.settings_show_input = false;

        // Update config with new API key
        self.update_config_from_settings();
    }

    /// Update config from settings
    fn update_config_from_settings(&mut self) {
        // Update API keys
        self.config.llm.openai_api_key =
            self.settings.openai.api_key.clone().unwrap_or_default();
        self.config.llm.anthropic_api_key =
            self.settings.anthropic.api_key.clone().unwrap_or_default();
        self.config.llm.google_api_key =
            self.settings.google.api_key.clone().unwrap_or_default();
        self.config.llm.openrouter_api_key =
            self.settings.openrouter.api_key.clone().unwrap_or_default();
        
        // Update provider
        self.config.llm.default_provider = self.settings.default_provider.to_string();
        
        // Update model based on selected provider's default model
        // This is CRITICAL - without this, the model stays as "gpt-4" which other providers don't recognize
        use crate::settings::Provider;
        self.config.llm.default_model = match self.settings.default_provider {
            Provider::OpenAI => self.settings.openai.default_model.clone()
                .unwrap_or_else(|| "gpt-4o".to_string()),
            Provider::Anthropic => self.settings.anthropic.default_model.clone()
                .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string()),
            Provider::Google => self.settings.google.default_model.clone()
                .unwrap_or_else(|| "gemini-2.0-flash".to_string()),
            Provider::OpenRouter => self.settings.openrouter.default_model.clone()
                .unwrap_or_else(|| "anthropic/claude-sonnet-4".to_string()),
        };
    }

    /// Scroll to bottom of messages
    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.max_scroll;
    }

    /// Update max scroll based on content
    pub fn update_scroll_bounds(&mut self, content_height: u16, viewport_height: u16) {
        self.max_scroll = content_height.saturating_sub(viewport_height);
        if self.scroll_offset > self.max_scroll {
            self.scroll_offset = self.max_scroll;
        }
    }
}
