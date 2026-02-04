//! Application State
//!
//! Contains the main application state and logic for the TUI.

use crate::agents::{self, LiteratureResult, PlanningResult};
use crate::analysis::{AnalysisConfig, build_manuscript, run_analysis};
use crate::config::Config;
use crate::data_registry::{DatasetRecord, DatasetRegistry};
use crate::models::UploadedDataset;
use crate::settings::{SettingsStorage, UserSettings};
use crate::tui::event::AppAction;
use chrono::{DateTime, Utc};
use std::time::Instant;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use tui_textarea::TextArea;
use uuid::Uuid;

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

#[derive(Debug, Clone, PartialEq)]
pub enum WorkflowStage {
    Upload,
    Planning,
    Literature,
    Findings,
    ResearcherFeedback,
    Draft1,
    UserFeedback1,
    Draft2,
    UserFeedback2,
    Draft3,
    UserFeedback3,
    LatexReady,
    Complete,
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
    /// Workflow stage updated
    WorkflowStageUpdated(WorkflowStage),
    /// Add a message to the chat
    WorkflowMessage(MessageRole, String),
}

/// Provider configuration for settings view
#[derive(Debug, Clone)]
pub struct ProviderField {
    pub id: &'static str,
    pub name: &'static str,
    pub has_key: bool,
    pub key_hint: Option<String>,
}

/// API status indicator
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ApiStatus {
    /// API is configured and ready
    Ready,
    /// API is not configured
    NotConfigured,
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

    // API Status (for status indicators)
    pub llm_status: ApiStatus,
    pub search_status: ApiStatus,

    // Streaming stats
    pub stream_start: Option<Instant>,
    pub stream_tokens: usize,
    pub stream_tps: f32,
    pub last_stream_tps: f32,
    pub spinner_index: usize,

    // Guided workflow state
    pub workflow_stage: WorkflowStage,
    pub planning_result: Option<PlanningResult>,
    pub literature_results: Vec<LiteratureResult>,
    pub findings_summary: Option<String>,
    pub manuscript_base: Option<String>,
    pub draft_versions: Vec<String>,
    pub feedbacks: Vec<String>,
    pub latex_output: Option<String>,
    pub auto_mode: bool,

    // Async communication
    event_rx: Option<mpsc::Receiver<AppEvent>>,
    event_tx: Option<mpsc::Sender<AppEvent>>,

    // Local dataset state for TUI workflows
    pub dataset_registry: DatasetRegistry,
    pub last_dataset_id: Option<String>,
}

impl App {
    /// Create a new application instance
    pub fn new(config: Config) -> Self {
        // Initialize text input
        let mut input = TextArea::default();
        input.set_cursor_line_style(ratatui::style::Style::default());
        input.set_placeholder_text("Paste dataset path (CSV/TSV with Ensembl ID + Age columns)...");

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

        // Add welcome message (will be updated with API status below)
        let messages = vec![ChatMessage {
            role: MessageRole::System,
            content: "Welcome to Oxidized Bio Research Agent!\n\n\
                     Initializing..."
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
            llm_status: ApiStatus::NotConfigured,
            search_status: ApiStatus::NotConfigured,
            stream_start: None,
            stream_tokens: 0,
            stream_tps: 0.0,
            last_stream_tps: 0.0,
            spinner_index: 0,
            workflow_stage: WorkflowStage::Upload,
            planning_result: None,
            literature_results: Vec::new(),
            findings_summary: None,
            manuscript_base: None,
            draft_versions: Vec::new(),
            feedbacks: Vec::new(),
            latex_output: None,
            auto_mode: true,
            event_rx: Some(rx),
            event_tx: Some(tx),
            dataset_registry: DatasetRegistry::default(),
            last_dataset_id: None,
        };

        app.update_config_from_settings();
        app.update_api_status();
        
        // Update welcome message with actual API status
        let llm_status_str = if app.config.llm.active_api_key().is_some() {
            format!("LLM: ✓ {} configured", app.config.llm.default_provider)
        } else {
            "LLM: ✗ Not configured".to_string()
        };
        
        let search_status_str = if app.config.search.serpapi_available() {
            "SerpAPI: ✓ Configured".to_string()
        } else {
            "SerpAPI: ✗ Not configured".to_string()
        };
        
        app.messages[0].content = format!(
            "Welcome to Oxidized Bio Research Agent!\n\n\
             API Status: {} | {}\n\n\
             AUTOMATED WORKFLOW\n\
             Paste a dataset path (.csv or .tsv) to begin automated analysis:\n\
             → Upload → Plan → Literature → Findings → Drafts 1-3 → LaTeX\n\n\
             Requirements: Dataset must include Ensembl ID and Age columns.\n\n\
             Examples:\n\
             • /home/user/data/microarray.csv\n\
             • ~/Documents/experiment_data.tsv\n\
             • ./data/samples.csv\n\n\
             Tip: You can drag & drop a file into the terminal or use tab completion.\n\n\
             Commands: Type /help for manual commands | Ctrl+S for Settings",
            llm_status_str, search_status_str
        );
        
        app
    }

    fn reset_stream_stats(&mut self) {
        self.stream_start = None;
        self.stream_tokens = 0;
        self.stream_tps = 0.0;
    }

    fn estimate_tokens(text: &str) -> usize {
        let chars = text.chars().count();
        if chars == 0 { 0 } else { (chars + 3) / 4 }
    }

    /// Update API status indicators based on current configuration
    pub fn update_api_status(&mut self) {
        // Check LLM status - any provider key configured
        self.llm_status = if self.config.llm.active_api_key().is_some() {
            ApiStatus::Ready
        } else {
            ApiStatus::NotConfigured
        };

        // Check SerpAPI status
        self.search_status = if self.config.search.serpapi_available() {
            ApiStatus::Ready
        } else {
            ApiStatus::NotConfigured
        };
    }

    /// Build the provider list from settings
    fn build_provider_list(settings: &UserSettings) -> Vec<ProviderField> {
        vec![
            // LLM Providers
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
            ProviderField {
                id: "groq",
                name: "Groq Cloud",
                has_key: settings.groq.api_key.is_some(),
                key_hint: settings
                    .groq
                    .api_key
                    .as_ref()
                    .map(|k| format!("••••{}", &k[k.len().saturating_sub(4)..])),
            },
            // Search API (SerpAPI for Google Scholar/Light)
            ProviderField {
                id: "serpapi",
                name: "SerpAPI (Scholar/Search)",
                has_key: settings.search.serpapi_key.is_some(),
                key_hint: settings
                    .search
                    .serpapi_key
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
            AppEvent::WorkflowStageUpdated(stage) => {
                self.workflow_stage = stage;
            }
            AppEvent::WorkflowMessage(role, content) => {
                self.messages.push(ChatMessage {
                    role,
                    content,
                    timestamp: Utc::now(),
                });
            }
            AppEvent::ObjectiveUpdated(objective) => {
                self.current_objective = Some(objective);
            }
            AppEvent::ResponseChunk(chunk) => {
                if self.stream_start.is_none() {
                    self.stream_start = Some(Instant::now());
                }
                self.stream_tokens = self.stream_tokens.saturating_add(Self::estimate_tokens(&chunk));
                if let Some(start) = self.stream_start {
                    let elapsed = start.elapsed().as_secs_f32().max(0.001);
                    self.stream_tps = self.stream_tokens as f32 / elapsed;
                }

                // Append to last assistant message if streaming
                if let Some(last) = self.messages.last_mut() {
                    if last.role == MessageRole::Assistant {
                        last.content.push_str(&chunk);
                        return;
                    }
                }

                self.messages.push(ChatMessage {
                    role: MessageRole::Assistant,
                    content: chunk,
                    timestamp: Utc::now(),
                });
            }
            AppEvent::ResponseComplete(response) => {
                if self.stream_start.is_some() {
                    self.last_stream_tps = self.stream_tps;
                    self.reset_stream_stats();
                }
                if let Some(last) = self.messages.last_mut() {
                    if last.role == MessageRole::Assistant {
                        last.content = response;
                    } else {
                        self.messages.push(ChatMessage {
                            role: MessageRole::Assistant,
                            content: response,
                            timestamp: Utc::now(),
                        });
                    }
                } else {
                    self.messages.push(ChatMessage {
                        role: MessageRole::Assistant,
                        content: response,
                        timestamp: Utc::now(),
                    });
                }
                self.pipeline_stage = PipelineStage::Complete;
                self.scroll_to_bottom();
            }
            AppEvent::Error(error) => {
                self.reset_stream_stats();
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
                if matches!(self.pipeline_stage, PipelineStage::Generating) {
                    self.spinner_index = self.spinner_index.wrapping_add(1);
                } else {
                    self.spinner_index = 0;
                }
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
        // Update placeholder based on workflow stage
        let placeholder = match self.workflow_stage {
            WorkflowStage::Upload => "Paste dataset path (CSV/TSV with Ensembl ID + Age columns)...",
            _ => "Type a question or /help for commands...",
        };
        self.input.set_placeholder_text(placeholder);

        // Add user message
        self.messages.push(ChatMessage {
            role: MessageRole::User,
            content: content.clone(),
            timestamp: Utc::now(),
        });

        if self.handle_slash_command(&content).await {
            self.scroll_to_bottom();
            return;
        }

        if self.auto_mode && self.workflow_stage == WorkflowStage::Upload {
            match self
                .load_dataset_from_path(&content, None)
                .await
            {
                Ok(record) => {
                    self.last_dataset_id = Some(record.dataset.id.clone());
                    self.dataset_registry.insert(record.clone()).await;
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!(
                            "Dataset loaded: {}\nRows: {} | Columns: {}\nID: {}\nAuto workflow starting...",
                            record.dataset.filename,
                            record.row_count,
                            record.columns.len(),
                            record.dataset.id
                        ),
                        timestamp: Utc::now(),
                    });
                    let tx = self.event_tx.clone().unwrap();
                    let config = self.config.clone();
                    tokio::spawn(async move {
                        Self::run_automated_workflow(record, config, tx).await;
                    });
                    self.workflow_stage = WorkflowStage::Planning;
                }
                Err(e) => {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("Upload failed: {}", e),
                        timestamp: Utc::now(),
                    });
                }
            }
            self.scroll_to_bottom();
            return;
        }

        // Check if any API key is configured (in settings)
        let has_llm_key = self.settings.openai.api_key.is_some()
            || self.settings.anthropic.api_key.is_some()
            || self.settings.google.api_key.is_some()
            || self.settings.openrouter.api_key.is_some()
            || self.settings.groq.api_key.is_some();
        
        let has_serpapi_key = self.settings.search.serpapi_key.is_some();

        // Also verify the config has the keys (should match settings)
        let config_has_llm = self.config.llm.active_api_key().is_some();
        let config_has_serpapi = self.config.search.serpapi_available();

        if !has_llm_key && !has_serpapi_key {
            self.messages.push(ChatMessage {
                role: MessageRole::System,
                content: "No API key configured. Press Ctrl+S to open Settings and add an API key."
                    .to_string(),
                timestamp: Utc::now(),
            });
            return;
        }

        // Debug: Show if there's a mismatch between settings and config
        if (has_llm_key && !config_has_llm) || (has_serpapi_key && !config_has_serpapi) {
            warn!(
                "Config mismatch - Settings has LLM: {}, Config has LLM: {}, Settings has SerpAPI: {}, Config has SerpAPI: {}",
                has_llm_key, config_has_llm, has_serpapi_key, config_has_serpapi
            );
            // Force update config from settings
            self.update_config_from_settings();
            self.update_api_status();
        }

        // Start research pipeline
        self.pipeline_stage = PipelineStage::Planning;
        self.current_objective = None;
        self.reset_stream_stats();

        // Get event sender
        let tx = self.event_tx.clone().unwrap();
        let config = self.config.clone();

        // Spawn async research task
        tokio::spawn(async move {
            Self::run_research_pipeline(content, config, tx).await;
        });

        self.scroll_to_bottom();
    }

    async fn handle_slash_command(&mut self, content: &str) -> bool {
        if !content.starts_with('/') {
            return false;
        }

        let mut parts = content.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        match cmd {
            "/help" => {
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: "Commands:\n\
/upload <path> [description]\n\
/list (list loaded datasets)\n\
/use <dataset_id>\n\
/analyze [dataset_id] [target=age] [group=cell_type] [box=marker_1] [cov=batch,sex]\n\
 /status (show workflow stage)\n\
 /next (advance workflow stage)\n\
 /feedback <text>\n\
 /latex (render LaTeX for latest draft)\n\
Tip: run /upload first, then /analyze."
                        .to_string(),
                    timestamp: Utc::now(),
                });
                return true;
            }
            "/upload" => {
                let path = parts.next();
                if path.is_none() {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: "Usage: /upload <path> [description]".to_string(),
                        timestamp: Utc::now(),
                    });
                    return true;
                }
                let description = parts.collect::<Vec<_>>().join(" ");
                match self
                    .load_dataset_from_path(
                        path.unwrap(),
                        if description.is_empty() { None } else { Some(description) },
                    )
                    .await
                {
                    Ok(record) => {
                        self.last_dataset_id = Some(record.dataset.id.clone());
                        self.dataset_registry.insert(record.clone()).await;
                        self.workflow_stage = WorkflowStage::Planning;
                        self.messages.push(ChatMessage {
                            role: MessageRole::System,
                            content: format!(
                                "Dataset loaded: {}\nRows: {} | Columns: {}\nID: {}",
                                record.dataset.filename,
                                record.row_count,
                                record.columns.len(),
                                record.dataset.id
                            ),
                            timestamp: Utc::now(),
                        });
                    }
                    Err(e) => {
                        self.messages.push(ChatMessage {
                            role: MessageRole::System,
                            content: format!("Upload failed: {}", e),
                            timestamp: Utc::now(),
                        });
                    }
                }
                return true;
            }
            "/list" => {
                let mut list = String::new();
                if let Some(last_id) = &self.last_dataset_id {
                    list.push_str(&format!("Active dataset: {}\n", last_id));
                }
                let datasets = self.dataset_registry.snapshot().await;
                if datasets.is_empty() {
                    list.push_str("No datasets loaded.");
                } else {
                    for record in datasets {
                        list.push_str(&format!(
                            "- {} ({}, rows: {})\n",
                            record.dataset.id,
                            record.dataset.filename,
                            record.row_count
                        ));
                    }
                }
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: list,
                    timestamp: Utc::now(),
                });
                return true;
            }
            "/use" => {
                if let Some(id) = parts.next() {
                    self.last_dataset_id = Some(id.to_string());
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("Active dataset set to {}", id),
                        timestamp: Utc::now(),
                    });
                } else {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: "Usage: /use <dataset_id>".to_string(),
                        timestamp: Utc::now(),
                    });
                }
                return true;
            }
            "/analyze" => {
                let dataset_id = parts
                    .next()
                    .map(|s| s.to_string())
                    .or_else(|| self.last_dataset_id.clone());
                if dataset_id.is_none() {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: "Usage: /analyze <dataset_id> [target=age] [group=cell_type] [box=marker_1] [cov=batch,sex]".to_string(),
                        timestamp: Utc::now(),
                    });
                    return true;
                }
                let mut target = "age".to_string();
                let mut group = "cell_type".to_string();
                let mut boxplot = None;
                let mut covariates: Vec<String> = Vec::new();
                for part in parts {
                    if let Some((k, v)) = part.split_once('=') {
                        match k {
                            "target" => target = v.to_string(),
                            "group" => group = v.to_string(),
                            "box" => boxplot = Some(v.to_string()),
                            "cov" => {
                                covariates = v
                                    .split(',')
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                            }
                            _ => {}
                        }
                    }
                }
                let dataset_id = dataset_id.unwrap();
                match self.dataset_registry.get(&dataset_id).await {
                    Some(record) => {
                        let output_dir = std::path::Path::new("artifacts")
                            .join("analysis")
                            .join(&dataset_id);
                        if let Err(e) = tokio::fs::create_dir_all(&output_dir).await {
                            self.messages.push(ChatMessage {
                                role: MessageRole::System,
                                content: format!("Failed to create artifacts dir: {}", e),
                                timestamp: Utc::now(),
                            });
                            return true;
                        }
                        let config = AnalysisConfig {
                            target_column: Some(target.clone()),
                            group_column: Some(group.clone()),
                            covariates,
                            boxplot_column: boxplot,
                            max_columns: 50,
                            max_groups: 20,
                        };
                        match run_analysis(&record, &config, &output_dir) {
                            Ok(result) => {
                                let manuscript = build_manuscript(
                                    &dataset_id,
                                    &target,
                                    &group,
                                    &record,
                                    &result,
                                );
                                let top = result
                                    .biomarker_candidates
                                    .iter()
                                    .take(10)
                                    .map(|b| format!("- {} (r={:.3})", b.column, b.correlation))
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                self.messages.push(ChatMessage {
                                    role: MessageRole::Assistant,
                                    content: format!("{}\n\nTop biomarkers:\n{}", manuscript, top),
                                    timestamp: Utc::now(),
                                });
                            }
                            Err(e) => {
                                self.messages.push(ChatMessage {
                                    role: MessageRole::System,
                                    content: format!("Analysis failed: {}", e),
                                    timestamp: Utc::now(),
                                });
                            }
                        }
                    }
                    None => {
                        self.messages.push(ChatMessage {
                            role: MessageRole::System,
                            content: format!("Dataset not found: {}", dataset_id),
                            timestamp: Utc::now(),
                        });
                    }
                }
                return true;
            }
            "/status" => {
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: format!("Workflow stage: {:?}", self.workflow_stage),
                    timestamp: Utc::now(),
                });
                return true;
            }
            "/next" => {
                self.advance_workflow().await;
                return true;
            }
            "/feedback" => {
                let feedback = parts.collect::<Vec<_>>().join(" ");
                if feedback.is_empty() {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: "Usage: /feedback <text>".to_string(),
                        timestamp: Utc::now(),
                    });
                    return true;
                }
                self.feedbacks.push(feedback.clone());
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: "Feedback recorded. Use /next to apply it.".to_string(),
                    timestamp: Utc::now(),
                });
                return true;
            }
            "/latex" => {
                if let Some(draft) = self.draft_versions.last() {
                    let latex = self.render_latex(draft);
                    self.latex_output = Some(latex.clone());
                    self.workflow_stage = WorkflowStage::LatexReady;
                    self.messages.push(ChatMessage {
                        role: MessageRole::Assistant,
                        content: format!("LaTeX output:\n\n{}", latex),
                        timestamp: Utc::now(),
                    });
                } else {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: "No draft available yet. Use /next to generate drafts.".to_string(),
                        timestamp: Utc::now(),
                    });
                }
                return true;
            }
            _ => {
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: format!("Unknown command: {}", cmd),
                    timestamp: Utc::now(),
                });
                return true;
            }
        }
    }

    async fn load_dataset_from_path(
        &self,
        path: &str,
        description: Option<String>,
    ) -> Result<DatasetRecord, String> {
        // Clean up the path: trim whitespace, expand home directory
        let path = path.trim();
        
        // Expand ~ to home directory
        let expanded_path = if path.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                home.join(&path[2..])
            } else {
                std::path::PathBuf::from(path)
            }
        } else if path == "~" {
            dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(path))
        } else {
            std::path::PathBuf::from(path)
        };
        
        // Convert to absolute path if relative
        let absolute_path = if expanded_path.is_absolute() {
            expanded_path
        } else {
            std::env::current_dir()
                .map_err(|e| format!("Failed to get current directory: {}", e))?
                .join(&expanded_path)
        };
        
        // Check if file exists before trying to read
        if !absolute_path.exists() {
            return Err(format!(
                "File not found: {}\n\nPlease check:\n\
                 1. The file path is correct\n\
                 2. The file exists at that location\n\
                 3. You have permission to read the file",
                absolute_path.display()
            ));
        }
        
        if !absolute_path.is_file() {
            return Err(format!(
                "Path is not a file: {}\n\nPlease provide a path to a .csv or .tsv file.",
                absolute_path.display()
            ));
        }
        
        let extension = absolute_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if extension != "csv" && extension != "tsv" {
            return Err(format!(
                "Only .csv or .tsv files are supported.\nYour file has extension: .{}",
                extension
            ));
        }
        let delimiter = if extension == "tsv" { b'\t' } else { b',' };
        let bytes = tokio::fs::read(&absolute_path)
            .await
            .map_err(|e| format!("Failed to read file {}: {}", absolute_path.display(), e))?;
        let dataset_id = Uuid::new_v4().to_string();
        let upload_dir = std::path::Path::new("uploads");
        tokio::fs::create_dir_all(upload_dir)
            .await
            .map_err(|e| format!("Failed to create uploads directory: {}", e))?;
        let filename = absolute_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("dataset.csv")
            .to_string();
        let stored_name = format!("{}-{}", dataset_id, filename);
        let local_path = upload_dir.join(&stored_name);
        tokio::fs::write(&local_path, &bytes)
            .await
            .map_err(|e| e.to_string())?;

        let (columns, row_count) = infer_csv_metadata(&bytes, delimiter)?;
        validate_microarray_headers(&columns)?;

        let dataset = UploadedDataset {
            filename: filename.clone(),
            id: dataset_id.clone(),
            description: description.unwrap_or_else(|| format!("Uploaded dataset {}", filename)),
            path: Some(local_path.to_string_lossy().to_string()),
            content: None,
            size: Some(bytes.len() as i64),
        };

        Ok(DatasetRecord {
            dataset,
            local_path: local_path.to_string_lossy().to_string(),
            content_type: "text/plain".to_string(),
            delimiter,
            has_headers: true,
            columns,
            row_count,
        })
    }

    async fn advance_workflow(&mut self) {
        match self.workflow_stage {
            WorkflowStage::Upload => {
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: "Please upload a dataset first using /upload.".to_string(),
                    timestamp: Utc::now(),
                });
            }
            WorkflowStage::Planning => {
                if let Err(e) = self.run_planning_stage().await {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("Planning failed: {}", e),
                        timestamp: Utc::now(),
                    });
                } else {
                    self.workflow_stage = WorkflowStage::Literature;
                }
            }
            WorkflowStage::Literature => {
                if let Err(e) = self.run_literature_stage().await {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("Literature review failed: {}", e),
                        timestamp: Utc::now(),
                    });
                } else {
                    self.workflow_stage = WorkflowStage::Findings;
                }
            }
            WorkflowStage::Findings => {
                if let Err(e) = self.run_findings_stage().await {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("Findings generation failed: {}", e),
                        timestamp: Utc::now(),
                    });
                } else {
                    self.workflow_stage = WorkflowStage::ResearcherFeedback;
                }
            }
            WorkflowStage::ResearcherFeedback | WorkflowStage::Draft1 => {
                let draft = self.build_draft(1);
                self.draft_versions.push(draft.clone());
                self.messages.push(ChatMessage {
                    role: MessageRole::Assistant,
                    content: format!("Draft 1:\n\n{}", draft),
                    timestamp: Utc::now(),
                });
                self.workflow_stage = WorkflowStage::UserFeedback1;
            }
            WorkflowStage::UserFeedback1 | WorkflowStage::Draft2 => {
                let draft = self.build_draft(2);
                self.draft_versions.push(draft.clone());
                self.messages.push(ChatMessage {
                    role: MessageRole::Assistant,
                    content: format!("Draft 2:\n\n{}", draft),
                    timestamp: Utc::now(),
                });
                self.workflow_stage = WorkflowStage::UserFeedback2;
            }
            WorkflowStage::UserFeedback2 | WorkflowStage::Draft3 => {
                let draft = self.build_draft(3);
                self.draft_versions.push(draft.clone());
                self.messages.push(ChatMessage {
                    role: MessageRole::Assistant,
                    content: format!("Draft 3:\n\n{}", draft),
                    timestamp: Utc::now(),
                });
                self.workflow_stage = WorkflowStage::UserFeedback3;
            }
            WorkflowStage::UserFeedback3 => {
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: "Ready for LaTeX output. Run /latex to export, or provide more /feedback and /next.".to_string(),
                    timestamp: Utc::now(),
                });
                self.workflow_stage = WorkflowStage::LatexReady;
            }
            WorkflowStage::LatexReady | WorkflowStage::Complete => {
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: "Workflow complete. Use /latex to re-render the latest draft or /feedback to continue refining.".to_string(),
                    timestamp: Utc::now(),
                });
            }
        }
    }

    async fn run_planning_stage(&mut self) -> Result<(), String> {
        let dataset_id = self
            .last_dataset_id
            .clone()
            .ok_or_else(|| "No dataset loaded. Use /upload.".to_string())?;
        let record = self
            .dataset_registry
            .get(&dataset_id)
            .await
            .ok_or_else(|| "Dataset not found.".to_string())?;
        let prompt = format!(
            "Create a research plan to discover aging biomarkers from log2-normalized microarray data. \
Dataset has {} rows and {} columns. Ensure Ensembl IDs and age are primary variables.",
            record.row_count,
            record.columns.len()
        );
        match agents::PlanningAgent::generate_plan(&prompt, None, &self.config).await {
            Ok(plan) => {
                self.planning_result = Some(plan.clone());
                self.messages.push(ChatMessage {
                    role: MessageRole::Assistant,
                    content: format!("Research plan generated:\n{}", plan.current_objective),
                    timestamp: Utc::now(),
                });
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    async fn run_literature_stage(&mut self) -> Result<(), String> {
        let plan = self
            .planning_result
            .clone()
            .ok_or_else(|| "No plan available. Run /next after planning.".to_string())?;
        let mut results = Vec::new();
        for task in plan.plan.iter().filter(|t| t.task_type == "LITERATURE") {
            match agents::LiteratureAgent::execute_task(task, &self.config).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    return Err(format!("Literature task failed: {}", e));
                }
            }
        }
        self.literature_results = results;
        self.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: format!(
                "Literature review complete. Sources: {}",
                self.literature_results.len()
            ),
            timestamp: Utc::now(),
        });
        Ok(())
    }

    async fn run_findings_stage(&mut self) -> Result<(), String> {
        let dataset_id = self
            .last_dataset_id
            .clone()
            .ok_or_else(|| "No dataset loaded. Use /upload.".to_string())?;
        let record = self
            .dataset_registry
            .get(&dataset_id)
            .await
            .ok_or_else(|| "Dataset not found.".to_string())?;
        let output_dir = std::path::Path::new("artifacts")
            .join("analysis")
            .join(&dataset_id);
        tokio::fs::create_dir_all(&output_dir)
            .await
            .map_err(|e| e.to_string())?;
        let config = AnalysisConfig {
            target_column: Some("age".to_string()),
            group_column: Some("cell_type".to_string()),
            covariates: Vec::new(),
            boxplot_column: None,
            max_columns: 50,
            max_groups: 20,
        };
        let analysis = run_analysis(&record, &config, &output_dir).map_err(|e| e.to_string())?;
        let manuscript = build_manuscript(&dataset_id, "age", "cell_type", &record, &analysis);
        self.findings_summary = Some(analysis.summary.clone());
        self.manuscript_base = Some(manuscript.clone());
        self.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: format!("Findings generated.\n{}", analysis.summary),
            timestamp: Utc::now(),
        });
        Ok(())
    }

    fn build_draft(&self, version: usize) -> String {
        let base = self
            .manuscript_base
            .clone()
            .unwrap_or_else(|| "No manuscript base available.".to_string());
        let plan = self
            .planning_result
            .as_ref()
            .map(|p| p.current_objective.clone())
            .unwrap_or_else(|| "No plan available.".to_string());
        let literature = if self.literature_results.is_empty() {
            "No literature sources available.".to_string()
        } else {
            let items = self
                .literature_results
                .iter()
                .take(5)
                .map(|r| r.objective.clone())
                .collect::<Vec<_>>()
                .join(", ");
            format!("Key literature tasks: {}", items)
        };
        let feedback = if self.feedbacks.is_empty() {
            "No feedback provided.".to_string()
        } else {
            self.feedbacks.join(" | ")
        };

        format!(
            "Draft {version}\n\n{base}\n\nResearch Plan:\n{plan}\n\nLiterature Review:\n{literature}\n\nFeedback Incorporated:\n{feedback}\n"
        )
    }

    fn render_latex(&self, draft: &str) -> String {
        let mut latex = String::new();
        latex.push_str("\\documentclass{article}\n");
        latex.push_str("\\usepackage[margin=1in]{geometry}\n");
        latex.push_str("\\usepackage{graphicx}\n");
        latex.push_str("\\begin{document}\n");
        for line in draft.lines() {
            if line.trim().is_empty() {
                latex.push_str("\n\n");
            } else if line.starts_with("Draft") || line.starts_with("Project ID") || line.starts_with("Title") {
                latex.push_str(&format!("\\section*{{{}}}\n", line.replace("_", "\\_")));
            } else {
                latex.push_str(&format!("{}\\\\\n", line.replace("_", "\\_")));
            }
        }
        latex.push_str("\\end{document}\n");
        latex
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
                let tx_chunks = tx.clone();
                let response = agents::ReplyAgent::generate_response_streaming(
                    &message,
                    Some(&plan),
                    &literature_results,
                    reply_mode,
                    &config,
                    move |chunk| {
                        let _ = tx_chunks.try_send(AppEvent::ResponseChunk(chunk.to_string()));
                    },
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

    async fn run_automated_workflow(
        record: DatasetRecord,
        config: Config,
        tx: mpsc::Sender<AppEvent>,
    ) {
        let dataset_id = record.dataset.id.clone();

        let _ = tx
            .send(AppEvent::WorkflowStageUpdated(WorkflowStage::Planning))
            .await;
        let plan_prompt = format!(
            "Create a research plan to discover aging biomarkers from log2-normalized microarray data. \
Dataset has {} rows and {} columns. Ensure Ensembl IDs and age are primary variables.",
            record.row_count,
            record.columns.len()
        );
        let planning_result = agents::PlanningAgent::generate_plan(&plan_prompt, None, &config).await;
        let plan = match planning_result {
            Ok(plan) => {
                let _ = tx
                    .send(AppEvent::WorkflowMessage(
                        MessageRole::Assistant,
                        format!("Research plan generated:\n{}", plan.current_objective),
                    ))
                    .await;
                plan
            }
            Err(e) => {
                let _ = tx
                    .send(AppEvent::Error(format!("Planning failed: {}", e)))
                    .await;
                return;
            }
        };

        let _ = tx
            .send(AppEvent::WorkflowStageUpdated(WorkflowStage::Literature))
            .await;
        let mut literature_results = Vec::new();
        for task in plan.plan.iter().filter(|t| t.task_type == "LITERATURE") {
            match agents::LiteratureAgent::execute_task(task, &config).await {
                Ok(result) => literature_results.push(result),
                Err(e) => {
                    let _ = tx
                        .send(AppEvent::Error(format!("Literature task failed: {}", e)))
                        .await;
                    return;
                }
            }
        }
        let _ = tx
            .send(AppEvent::WorkflowMessage(
                MessageRole::Assistant,
                format!("Literature review complete. Sources: {}", literature_results.len()),
            ))
            .await;

        let _ = tx
            .send(AppEvent::WorkflowStageUpdated(WorkflowStage::Findings))
            .await;
        let output_dir = std::path::Path::new("artifacts")
            .join("analysis")
            .join(&dataset_id);
        if let Err(e) = tokio::fs::create_dir_all(&output_dir).await {
            let _ = tx
                .send(AppEvent::Error(format!("Failed to create artifacts dir: {}", e)))
                .await;
            return;
        }
        let analysis = match run_analysis(
            &record,
            &AnalysisConfig {
                target_column: Some("age".to_string()),
                group_column: Some("cell_type".to_string()),
                covariates: Vec::new(),
                boxplot_column: None,
                max_columns: 50,
                max_groups: 20,
            },
            &output_dir,
        ) {
            Ok(result) => result,
            Err(e) => {
                let _ = tx
                    .send(AppEvent::Error(format!("Analysis failed: {}", e)))
                    .await;
                return;
            }
        };
        let manuscript = build_manuscript(&dataset_id, "age", "cell_type", &record, &analysis);
        let _ = tx
            .send(AppEvent::WorkflowMessage(
                MessageRole::Assistant,
                format!("Findings generated.\n{}", analysis.summary),
            ))
            .await;

        let _ = tx
            .send(AppEvent::WorkflowStageUpdated(WorkflowStage::Draft1))
            .await;
        let draft1 = Self::build_automated_draft(1, &manuscript, &plan, &literature_results);
        let _ = tx
            .send(AppEvent::WorkflowMessage(
                MessageRole::Assistant,
                format!("Draft 1:\n\n{}", draft1),
            ))
            .await;

        let _ = tx
            .send(AppEvent::WorkflowStageUpdated(WorkflowStage::Draft2))
            .await;
        let draft2 = Self::build_automated_draft(2, &manuscript, &plan, &literature_results);
        let _ = tx
            .send(AppEvent::WorkflowMessage(
                MessageRole::Assistant,
                format!("Draft 2:\n\n{}", draft2),
            ))
            .await;

        let _ = tx
            .send(AppEvent::WorkflowStageUpdated(WorkflowStage::Draft3))
            .await;
        let draft3 = Self::build_automated_draft(3, &manuscript, &plan, &literature_results);
        let _ = tx
            .send(AppEvent::WorkflowMessage(
                MessageRole::Assistant,
                format!("Draft 3:\n\n{}", draft3),
            ))
            .await;

        let latex = Self::render_latex_static(&draft3);
        let _ = tx
            .send(AppEvent::WorkflowStageUpdated(WorkflowStage::LatexReady))
            .await;
        let _ = tx
            .send(AppEvent::WorkflowMessage(
                MessageRole::Assistant,
                format!("LaTeX output:\n\n{}", latex),
            ))
            .await;
    }

    fn build_automated_draft(
        version: usize,
        manuscript: &str,
        plan: &PlanningResult,
        literature_results: &[LiteratureResult],
    ) -> String {
        let literature = if literature_results.is_empty() {
            "No literature sources available.".to_string()
        } else {
            let items = literature_results
                .iter()
                .take(5)
                .map(|r| r.objective.clone())
                .collect::<Vec<_>>()
                .join(", ");
            format!("Key literature tasks: {}", items)
        };
        format!(
            "Draft {version}\n\n{manuscript}\n\nResearch Plan:\n{}\n\nLiterature Review:\n{literature}\n",
            plan.current_objective
        )
    }

    fn render_latex_static(draft: &str) -> String {
        let mut latex = String::new();
        latex.push_str("\\documentclass{article}\n");
        latex.push_str("\\usepackage[margin=1in]{geometry}\n");
        latex.push_str("\\usepackage{graphicx}\n");
        latex.push_str("\\begin{document}\n");
        for line in draft.lines() {
            if line.trim().is_empty() {
                latex.push_str("\n\n");
            } else if line.starts_with("Draft")
                || line.starts_with("Project ID")
                || line.starts_with("Title")
            {
                latex.push_str(&format!("\\section*{{{}}}\n", line.replace("_", "\\_")));
            } else {
                latex.push_str(&format!("{}\\\\\n", line.replace("_", "\\_")));
            }
        }
        latex.push_str("\\end{document}\n");
        latex
    }

    /// Save the current setting
    async fn save_current_setting(&mut self) {
        if !self.settings_show_input || self.settings_input.is_empty() {
            return;
        }

        let provider_id = self.providers[self.settings_field_index].id;
        let key = std::mem::take(&mut self.settings_input);

        // Handle SerpAPI separately (it's a search API, not LLM provider)
        if provider_id == "serpapi" {
            self.settings.search.serpapi_key = Some(key);
            // Ensure search engines are enabled when key is set
            self.settings.search.scholar_enabled = true;
            self.settings.search.light_enabled = true;
        } else {
            // Update settings (only one LLM provider key at a time)
            if crate::settings::Provider::from_id(provider_id).is_none() {
                return;
            }
            self.settings.set_single_provider_key(provider_id, key);
        }

        // Save to storage
        if let Err(e) = self.settings_storage.save(&self.settings).await {
            error!("Failed to save settings: {}", e);
            self.messages.push(ChatMessage {
                role: MessageRole::System,
                content: format!("Failed to save API key: {}", e),
                timestamp: Utc::now(),
            });
        } else {
            info!("Settings saved for: {}", provider_id);
        }

        // Refresh provider list
        self.refresh_providers();
        self.settings_show_input = false;

        // Update config with new API key
        self.update_config_from_settings();
        
        // Update API status indicators
        self.update_api_status();
        
        // Show confirmation with current status
        let llm_ok = self.config.llm.active_api_key().is_some();
        let search_ok = self.config.search.serpapi_available();
        self.messages.push(ChatMessage {
            role: MessageRole::System,
            content: format!(
                "API key saved for {}.\n\nCurrent status: LLM {} | SerpAPI {}",
                provider_id,
                if llm_ok { "✓" } else { "✗" },
                if search_ok { "✓" } else { "✗" }
            ),
            timestamp: Utc::now(),
        });
    }

    /// Update config from settings
    fn update_config_from_settings(&mut self) {
        // Update LLM API keys
        self.config.llm.openai_api_key =
            self.settings.openai.api_key.clone().unwrap_or_default();
        self.config.llm.anthropic_api_key =
            self.settings.anthropic.api_key.clone().unwrap_or_default();
        self.config.llm.google_api_key =
            self.settings.google.api_key.clone().unwrap_or_default();
        self.config.llm.openrouter_api_key =
            self.settings.openrouter.api_key.clone().unwrap_or_default();
        self.config.llm.groq_api_key =
            self.settings.groq.api_key.clone().unwrap_or_default();
        
        // Update LLM provider
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
            Provider::Groq => self.settings.groq.default_model.clone()
                .unwrap_or_else(|| "groq/compound".to_string()),
        };

        // Update Search API config (SerpAPI)
        self.config.search.serpapi_key =
            self.settings.search.serpapi_key.clone().unwrap_or_default();
        self.config.search.scholar_enabled = self.settings.search.scholar_enabled;
        self.config.search.light_enabled = self.settings.search.light_enabled;
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
    
    /// Calculate scroll bounds based on current messages
    /// Call this before rendering to ensure max_scroll is up to date
    pub fn calculate_scroll_bounds(&mut self, terminal_height: u16) {
        // Calculate content height:
        // Each message has: 1 line for role + content lines + 1 blank line
        let mut content_height: u16 = 0;
        
        for msg in &self.messages {
            content_height += 1; // Role line
            content_height += msg.content.lines().count() as u16; // Content lines
            content_height += 1; // Blank line
        }
        
        // Add typing indicator if generating
        if matches!(self.pipeline_stage, PipelineStage::Generating) {
            content_height += 1;
        }
        
        // Calculate viewport height (terminal - header - progress - input - status - borders)
        // Header: 3, Progress: 4, Input: 4, Status: 1, Borders: ~4
        let viewport_height = terminal_height.saturating_sub(16);
        
        // Update scroll bounds
        self.update_scroll_bounds(content_height, viewport_height);
    }
}

fn infer_csv_metadata(bytes: &[u8], delimiter: u8) -> Result<(Vec<String>, usize), String> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(true)
        .from_reader(bytes);

    let headers = rdr
        .headers()
        .map_err(|e| e.to_string())?
        .iter()
        .map(|h| h.to_string())
        .collect::<Vec<_>>();

    let mut row_count = 0usize;
    for record in rdr.records() {
        record.map_err(|e| e.to_string())?;
        row_count += 1;
    }
    Ok((headers, row_count))
}

fn validate_microarray_headers(headers: &[String]) -> Result<(), String> {
    let lowered: Vec<String> = headers.iter().map(|h| h.to_lowercase()).collect();
    let has_ensembl = lowered.iter().any(|h| h.contains("ensembl"));
    let has_age = lowered.iter().any(|h| h == "age" || h.contains("age"));
    if !has_ensembl || !has_age {
        return Err("Dataset must include Ensembl ID and Age columns.".to_string());
    }
    Ok(())
}
