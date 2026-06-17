//! Central TUI application state and event handling.
//!
//! `App` owns all view state. State changes happen here (`handle_event`, `refresh_data`);
//! rendering is render-only and lives in `ui.rs`.

// =========================================================================================================
// Imports
// =========================================================================================================

use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::data::{OrderInfo, Portfolio};
use crate::trading::markets::{MarketInfo, MarketService};
use crate::trading::ExecutionEngine;

// =========================================================================================================
// Types
// =========================================================================================================

/// Available tabs in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Orders,
    Markets,
    MarketDetail,
    Logs,
    Docs,
    AiChat,
    Settings,
}

impl Tab {
    pub fn next(&self) -> Self {
        match self {
            Tab::Dashboard => Tab::Orders,
            Tab::Orders => Tab::Markets,
            Tab::Markets => Tab::MarketDetail,
            Tab::MarketDetail => Tab::Logs,
            Tab::Logs => Tab::Docs,
            Tab::Docs => Tab::AiChat,
            Tab::AiChat => Tab::Settings,
            Tab::Settings => Tab::Dashboard,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Tab::Dashboard => Tab::Settings,
            Tab::Orders => Tab::Dashboard,
            Tab::Markets => Tab::Orders,
            Tab::MarketDetail => Tab::Markets,
            Tab::Logs => Tab::MarketDetail,
            Tab::Docs => Tab::Logs,
            Tab::AiChat => Tab::Docs,
            Tab::Settings => Tab::AiChat,
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Orders => "Orders",
            Tab::Markets => "Markets",
            Tab::MarketDetail => "Market Detail",
            Tab::Logs => "Logs",
            Tab::Docs => "Docs",
            Tab::AiChat => "AI Chat",
            Tab::Settings => "Settings",
        }
    }

    pub fn all() -> [Tab; 8] {
        [
            Tab::Dashboard,
            Tab::Orders,
            Tab::Markets,
            Tab::MarketDetail,
            Tab::Logs,
            Tab::Docs,
            Tab::AiChat,
            Tab::Settings,
        ]
    }
}

/// Input mode for command entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Command,
    QuitConfirmation,
    LeaveMarketConfirmation,
    /// Tab-bar navigation: arrows move the highlighted tab, Enter activates it, Esc cancels.
    TabNavigation,
}

/// Quit confirmation selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuitSelection {
    No, // Default
    Yes,
}

/// Leave market confirmation selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaveSelection {
    No, // Default
    Yes,
}

/// Log entry for the logs tab
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Success,
}

/// Market analysis data for real-time detection visualization
#[derive(Debug, Clone, Default)]
pub struct MarketAnalysis {
    pub volume_history: Vec<(i64, f64)>, // timestamp, volume
    pub current_velocity: Option<f64>,
    pub current_obi: Option<f64>,
    pub recent_events: Vec<crate::data::VolumeVelocityEvent>,
}

/// Main application state
pub struct App {
    pub db_pool: crate::data::DbPool,
    pub execution_engine: Arc<ExecutionEngine>,
    pub market_service: MarketService,
    pub current_tab: Tab,
    /// Tab highlighted by the cursor while in `InputMode::TabNavigation`.
    /// Only `current_tab` decides what content is shown; this is the pending selection.
    pub highlighted_tab: Tab,
    pub should_quit: bool,
    pub logs: Vec<LogEntry>,
    pub portfolio: Option<Portfolio>,
    pub active_orders: Vec<OrderInfo>,
    pub is_paused: bool,
    pub last_order_id: Option<String>,
    pub last_refresh: Instant,

    // Command input
    pub input_mode: InputMode,
    pub command_input: String,
    pub quit_selection: QuitSelection,
    pub leave_selection: LeaveSelection,

    // Markets
    pub available_markets: Vec<MarketInfo>,
    pub joined_markets: Vec<String>,
    pub watched_markets_info: Vec<MarketInfo>,
    pub market_search_query: String,
    pub selected_market_index: usize,
    pub selected_watched_market_index: usize,
    pub is_loading_markets: bool,

    // Market analysis
    pub market_analysis_data: std::collections::HashMap<String, MarketAnalysis>,

    // RNG state
    rng_state: u64,

    // Docs tab state
    pub docs_selected_section: usize,
    pub docs_viewing_content: bool,
    pub docs_scroll_offset: u16,

    // AI Chatbot
    // Channels for async communication
    pub chat_request_tx: Option<tokio::sync::mpsc::Sender<String>>,
    pub chat_response_rx: Option<tokio::sync::mpsc::Receiver<anyhow::Result<String>>>,
    pub chat_state: crate::tui::ChatState,

    // Settings
    pub settings_editor: crate::tui::SettingsEditor,
}

// =========================================================================================================
// Implementation
// =========================================================================================================

impl App {
    pub fn new(
        db_pool: crate::data::DbPool,
        execution_engine: Arc<ExecutionEngine>,
        config: &crate::config::SecureConfig,
    ) -> Self {
        let mut app = Self {
            db_pool,
            execution_engine,
            market_service: MarketService::new(),
            current_tab: Tab::Dashboard,
            highlighted_tab: Tab::Dashboard,
            should_quit: false,
            logs: Vec::new(),
            portfolio: None,
            active_orders: Vec::new(),
            is_paused: false,
            last_order_id: None,
            last_refresh: Instant::now(),
            input_mode: InputMode::Normal,
            command_input: String::new(),
            quit_selection: QuitSelection::No,
            leave_selection: LeaveSelection::No,
            available_markets: Vec::new(),
            joined_markets: Vec::new(),
            watched_markets_info: Vec::new(),
            market_search_query: String::new(),
            selected_market_index: 0,
            selected_watched_market_index: 0,
            is_loading_markets: false,
            market_analysis_data: std::collections::HashMap::new(),

            rng_state: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            docs_selected_section: 0,
            docs_viewing_content: false,
            docs_scroll_offset: 0,
            chat_request_tx: None,
            chat_response_rx: None, // Will be set by run_tui if AI is enabled
            chat_state: crate::tui::ChatState::new(),
            settings_editor: crate::tui::SettingsEditor::from_config(config),
        };

        app.add_log(LogLevel::Info, "TUI initialized successfully");
        app.add_log(LogLevel::Info, "Press ':' to enter command mode");
        app.add_log(LogLevel::Info, "Press 'S' to search markets");

        // Initialize AI chatbot if Gemini API key is available
        if let Some(ref api_key) = config.gemini_api_key {
            let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(10);
            let (resp_tx, resp_rx) = tokio::sync::mpsc::channel::<anyhow::Result<String>>(10);

            app.chat_request_tx = Some(tx);
            app.chat_response_rx = Some(resp_rx);

            let api_key_clone = api_key.clone();
            let personality = config.ai_personality;
            let db_pool = app.db_pool.clone();
            let execution_engine = app.execution_engine.clone();

            tokio::spawn(async move {
                let gemini_client = crate::ai::GeminiClient::new(api_key_clone);
                let mut chatbot = crate::ai::AiChatbot::new(
                    gemini_client,
                    personality,
                    db_pool,
                    execution_engine,
                );

                while let Some(msg) = rx.recv().await {
                    // Send to AI
                    let result = chatbot.send_message(msg).await;
                    // Extract message string from response for now
                    let response_str = result.map(|r| r.message);

                    if resp_tx.send(response_str).await.is_err() {
                        break; // Receiver dropped
                    }
                }
            });

            app.add_log(LogLevel::Success, "AI Chat ready (Tab 7)");
        } else {
            app.add_log(
                LogLevel::Warning,
                "AI Chat unavailable - no API key configured",
            );
        }

        // Load watched markets from database
        app.add_log(LogLevel::Info, "Loading watched markets...");
        app
    }

    /// Initialize watched markets - call this after creating App
    pub async fn init_watched_markets(&mut self) {
        match crate::trading::markets::load_watched_markets(&self.db_pool).await {
            Ok(markets) => {
                self.joined_markets = markets.iter().map(|m| m.id.clone()).collect();
                self.watched_markets_info = markets;
                self.add_log(
                    LogLevel::Success,
                    &format!("Loaded {} watched markets", self.joined_markets.len()),
                );
            }
            Err(e) => {
                self.add_log(
                    LogLevel::Error,
                    &format!("Failed to load watched markets: {}", e),
                );
            }
        }
    }

    pub fn add_log(&mut self, level: LogLevel, message: &str) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        self.logs.push(LogEntry {
            timestamp,
            level,
            message: message.to_string(),
        });

        // Keep only last 100 logs
        if self.logs.len() > 100 {
            self.logs.remove(0);
        }
    }

    pub async fn refresh_data(&mut self) {
        // Poll for AI chat responses
        // Use take() to detach receiver from self to appease borrow checker
        if let Some(mut rx) = self.chat_response_rx.take() {
            while let Ok(result) = rx.try_recv() {
                self.chat_state.waiting_for_ai = false;
                match result {
                    Ok(response_msg) => {
                        self.chat_state
                            .history
                            .push(("model".to_string(), response_msg));
                        self.add_log(LogLevel::Success, "AI responded");
                    }
                    Err(e) => {
                        let error_msg = format!("Detailed Error: {:#?}", e);
                        self.add_log(LogLevel::Error, "AI request failed. Check chat.");
                        self.chat_state
                            .history
                            .push(("model".to_string(), error_msg));
                    }
                }
            }
            // Put receiver back
            self.chat_response_rx = Some(rx);
        }

        // Refresh every 500ms
        if self.last_refresh.elapsed().as_millis() < 500 {
            return;
        }
        self.last_refresh = Instant::now();

        // Update paused state
        self.is_paused = self.execution_engine.is_paused().await;

        // Update last order ID
        self.last_order_id = self.execution_engine.get_last_order_id().await;

        // Update portfolio
        if let Ok(portfolio) = self.execution_engine.get_portfolio().await {
            self.portfolio = Some(portfolio);
        }

        // Update active orders
        if let Ok(orders) = self.execution_engine.get_active_orders().await {
            self.active_orders = orders;
        }

        // Simulate market analysis data updates
        self.simulate_market_data();
    }

    fn simulate_market_data(&mut self) {
        let mut rng_state = self.rng_state;

        // Clone market IDs to avoid borrowing self while mutating analysis data
        let markets: Vec<String> = self
            .watched_markets_info
            .iter()
            .map(|m| m.id.clone())
            .collect();

        let next_random = |state: &mut u64| -> f64 {
            *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            (*state as f64) / (u64::MAX as f64)
        };

        for market_id in markets {
            let entry = self
                .market_analysis_data
                .entry(market_id.clone())
                .or_default();

            // Initialize if empty
            if entry.current_velocity.is_none() {
                entry.current_velocity = Some(0.0);
            }
            if entry.current_obi.is_none() {
                entry.current_obi = Some(0.0);
            }

            // Random walk for Velocity (-2000 to +2000)
            let rnd = next_random(&mut rng_state);
            if let Some(vel) = entry.current_velocity {
                let change = (rnd - 0.5) * 200.0; // Change by up to +/- 100
                let new_vel = (vel + change).clamp(-2000.0, 2000.0);
                entry.current_velocity = Some(new_vel);

                // Add event if spike
                if new_vel.abs() > 1000.0 && next_random(&mut rng_state) > 0.95 {
                    entry.recent_events.insert(
                        0,
                        crate::data::VolumeVelocityEvent {
                            market_id: market_id.clone(),
                            velocity: new_vel,
                            volume_delta: change,
                            time_delta: 0.5,
                            timestamp: chrono::Utc::now().timestamp(),
                        },
                    );
                    if entry.recent_events.len() > 10 {
                        entry.recent_events.pop();
                    }
                }
            }

            // Random walk for OBI (-1.0 to 1.0)
            let rnd2 = next_random(&mut rng_state);
            if let Some(obi) = entry.current_obi {
                let change = (rnd2 - 0.5) * 0.1; // Change by up to +/- 0.05
                let new_obi = (obi + change).clamp(-1.0, 1.0);
                entry.current_obi = Some(new_obi);
            }
        }

        self.rng_state = rng_state;
    }

    pub async fn handle_event(&mut self, event: KeyEvent) -> Result<()> {
        match self.input_mode {
            InputMode::Command => self.handle_command_input(event).await,
            InputMode::QuitConfirmation => self.handle_quit_confirmation(event),
            InputMode::LeaveMarketConfirmation => self.handle_leave_confirmation(event).await,
            InputMode::TabNavigation => self.handle_tab_navigation(event),
            InputMode::Normal => self.handle_normal_input(event).await,
        }
    }

    /// Enter tab-bar navigation mode, starting the cursor on the current tab.
    fn enter_tab_navigation(&mut self) {
        self.highlighted_tab = self.current_tab;
        self.input_mode = InputMode::TabNavigation;
    }

    /// Tab-bar navigation: arrows/Tab move the highlighted tab, Enter activates it, Esc cancels.
    fn handle_tab_navigation(&mut self, event: KeyEvent) -> Result<()> {
        match event.code {
            // Move the highlight along the tab bar without changing the shown content.
            KeyCode::Right | KeyCode::Tab => {
                self.highlighted_tab = self.highlighted_tab.next();
            }
            KeyCode::Left | KeyCode::BackTab => {
                self.highlighted_tab = self.highlighted_tab.prev();
            }
            // Activate the highlighted tab and return to normal mode.
            KeyCode::Enter => {
                self.current_tab = self.highlighted_tab;
                self.input_mode = InputMode::Normal;
            }
            // Cancel without changing the active tab.
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            // Numeric keys highlight a tab directly (still requires Enter to activate).
            KeyCode::Char(c @ '1'..='8') => {
                let idx = (c as u8 - b'1') as usize;
                if let Some(tab) = Tab::all().get(idx) {
                    self.highlighted_tab = *tab;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_command_input(&mut self, event: KeyEvent) -> Result<()> {
        match event.code {
            KeyCode::Enter => {
                let command = self.command_input.clone();
                self.command_input.clear();
                self.input_mode = InputMode::Normal;
                self.execute_command(&command).await;
            }
            KeyCode::Esc => {
                self.command_input.clear();
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                self.command_input.pop();
            }
            KeyCode::Char(c) => {
                self.command_input.push(c);
            }
            _ => {}
        }
        Ok(())
    }

    async fn execute_command(&mut self, command: &str) {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        let cmd = parts[0].to_lowercase();
        let args: Vec<&str> = parts[1..].to_vec();

        match cmd.as_str() {
            "/search" | "search" | "/s" | "s" => {
                if args.is_empty() {
                    self.add_log(LogLevel::Warning, "Usage: /search <keyword>");
                } else {
                    let keyword = args.join(" ");
                    self.search_markets(&keyword).await;
                }
            }
            "/joinmarket" | "joinmarket" | "/join" | "join" | "/j" | "j" => {
                if args.is_empty() {
                    self.add_log(LogLevel::Warning, "Usage: /joinmarket <market_id or index>");
                } else {
                    let market_ref = args[0];
                    self.join_market(market_ref).await;
                }
            }
            "/leavemarkt" | "leavemarket" | "/leave" | "leave" | "/l" => {
                if args.is_empty() {
                    self.add_log(LogLevel::Warning, "Usage: /leavemarket <market_id>");
                } else {
                    self.leave_market(args[0]).await;
                }
            }
            "/trending" | "trending" | "/t" | "t" => {
                self.load_trending_markets().await;
            }
            "/help" | "help" | "/h" | "?" => {
                self.show_command_help();
            }
            _ => {
                self.add_log(LogLevel::Warning, &format!("Unknown command: {}", cmd));
                self.add_log(LogLevel::Info, "Type /help for available commands");
            }
        }
    }

    async fn search_markets(&mut self, keyword: &str) {
        self.add_log(
            LogLevel::Info,
            &format!("Searching markets: '{}'...", keyword),
        );
        self.market_search_query = keyword.to_string();
        self.is_loading_markets = true;
        self.current_tab = Tab::Markets;

        match self.market_service.search_markets(keyword, 50).await {
            Ok(markets) => {
                let count = markets.len();
                self.available_markets = markets;
                self.selected_market_index = 0;
                self.is_loading_markets = false;
                self.add_log(LogLevel::Success, &format!("Found {} markets", count));
            }
            Err(e) => {
                self.is_loading_markets = false;
                self.add_log(LogLevel::Error, &format!("Search failed: {}", e));
            }
        }
    }

    async fn load_trending_markets(&mut self) {
        self.add_log(LogLevel::Info, "Loading trending markets...");
        self.market_search_query = "Trending".to_string();
        self.is_loading_markets = true;
        self.current_tab = Tab::Markets;

        match self.market_service.get_trending_markets(20).await {
            Ok(markets) => {
                let count = markets.len();
                self.available_markets = markets;
                self.selected_market_index = 0;
                self.is_loading_markets = false;
                self.add_log(
                    LogLevel::Success,
                    &format!("Loaded {} trending markets", count),
                );
            }
            Err(e) => {
                self.is_loading_markets = false;
                self.add_log(LogLevel::Error, &format!("Failed to load trending: {}", e));
            }
        }
    }

    async fn join_market(&mut self, market_ref: &str) {
        // Check if it's an index number
        if let Ok(index) = market_ref.parse::<usize>() {
            if index > 0 && index <= self.available_markets.len() {
                let market = self.available_markets[index - 1].clone();
                let market_id = market.id.clone();
                let question = market.question.clone();

                if !self.joined_markets.contains(&market_id) {
                    // Save to database
                    if let Err(e) =
                        crate::trading::markets::save_watched_market(&self.db_pool, &market).await
                    {
                        self.add_log(LogLevel::Error, &format!("Failed to save market: {}", e));
                        return;
                    }

                    self.joined_markets.push(market_id.clone());
                    self.watched_markets_info.push(market);
                    self.add_log(LogLevel::Success, &format!("Joined market: {}", question));
                    self.add_log(LogLevel::Info, &format!("ID: {}", market_id));
                } else {
                    self.add_log(LogLevel::Warning, "Already monitoring this market");
                }
                return;
            } else {
                self.add_log(
                    LogLevel::Error,
                    &format!(
                        "Invalid index: {}. Use 1-{}",
                        index,
                        self.available_markets.len()
                    ),
                );
                return;
            }
        }

        // Otherwise treat as market ID - try to find in available markets
        let market_id = market_ref.to_string();
        if !self.joined_markets.contains(&market_id) {
            // Try to find the market in available_markets
            if let Some(market) = self
                .available_markets
                .iter()
                .find(|m| m.id == market_id)
                .cloned()
            {
                if let Err(e) =
                    crate::trading::markets::save_watched_market(&self.db_pool, &market).await
                {
                    self.add_log(LogLevel::Error, &format!("Failed to save market: {}", e));
                    return;
                }
                self.watched_markets_info.push(market.clone());
            }

            self.joined_markets.push(market_id.clone());
            self.add_log(LogLevel::Success, &format!("Joined market: {}", market_id));
        } else {
            self.add_log(LogLevel::Warning, "Already monitoring this market");
        }
    }

    async fn leave_market(&mut self, market_id: &str) {
        if let Some(pos) = self.joined_markets.iter().position(|m| m == market_id) {
            // Remove from database
            if let Err(e) =
                crate::trading::markets::remove_watched_market(&self.db_pool, market_id).await
            {
                self.add_log(LogLevel::Error, &format!("Failed to remove from DB: {}", e));
                return;
            }

            self.joined_markets.remove(pos);
            self.watched_markets_info.retain(|m| m.id != market_id);
            self.add_log(LogLevel::Info, &format!("Left market: {}", market_id));
        } else {
            self.add_log(
                LogLevel::Warning,
                &format!("Not monitoring market: {}", market_id),
            );
        }
    }

    fn show_command_help(&mut self) {
        self.add_log(LogLevel::Info, "─── Available Commands ───");
        self.add_log(LogLevel::Info, "/search <keyword>  - Search markets");
        self.add_log(LogLevel::Info, "/trending          - Show trending markets");
        self.add_log(
            LogLevel::Info,
            "/joinmarket <id|#> - Join market by ID or index",
        );
        self.add_log(LogLevel::Info, "/leavemarket <id>  - Leave a market");
        self.add_log(LogLevel::Info, "/help              - Show this help");
    }

    async fn handle_normal_input(&mut self, event: KeyEvent) -> Result<()> {
        // Handle AI Chat tab navigation specially
        if self.current_tab == Tab::AiChat {
            if let Some(ref tx) = self.chat_request_tx {
                match event.code {
                    // ENTER: Toggle input mode or send message
                    KeyCode::Enter => {
                        if self.chat_state.input_active {
                            // Send message if not empty
                            if !self.chat_state.input.trim().is_empty() {
                                let user_message = self.chat_state.input.clone();
                                self.chat_state.clear_input();
                                self.chat_state.waiting_for_ai = true;
                                self.chat_state.input_active = false;

                                // Update local history
                                self.chat_state
                                    .history
                                    .push(("user".to_string(), user_message.clone()));

                                // Send to AI background task
                                let tx_clone = tx.clone();
                                tokio::spawn(async move {
                                    let _ = tx_clone.send(user_message).await;
                                });
                            } else {
                                // Empty message - just deactivate input
                                self.chat_state.input_active = false;
                            }
                        } else {
                            // Activate input mode
                            self.chat_state.input_active = true;
                        }
                        return Ok(());
                    }
                    // ESC: Cancel input mode
                    KeyCode::Esc => {
                        if self.chat_state.input_active {
                            self.chat_state.input_active = false;
                            self.chat_state.clear_input();
                        }
                        return Ok(());
                    }
                    // Backspace: Delete character (only in input mode)
                    KeyCode::Backspace if self.chat_state.input_active => {
                        self.chat_state.handle_backspace();
                        return Ok(());
                    }
                    // Text input (only in input mode)
                    KeyCode::Char(c) if self.chat_state.input_active => {
                        self.chat_state.handle_char(c);
                        return Ok(());
                    }
                    // Global navigation (only when NOT in input mode): enter tab-bar nav mode.
                    KeyCode::Tab | KeyCode::Right | KeyCode::Left | KeyCode::BackTab
                        if !self.chat_state.input_active =>
                    {
                        self.enter_tab_navigation();
                        return Ok(());
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') if !self.chat_state.input_active => {
                        self.input_mode = InputMode::QuitConfirmation;
                        self.quit_selection = QuitSelection::No;
                        return Ok(());
                    }
                    // Other shortcuts (only when NOT in input mode)
                    _ if !self.chat_state.input_active => {
                        // Fall through to global shortcuts below
                    }
                    // In input mode, ignore other keys
                    _ => {
                        return Ok(());
                    }
                }
            }
        }

        // Handle Settings tab navigation specially
        if self.current_tab == Tab::Settings {
            match event.code {
                // Up/Down for field navigation (only when NOT editing)
                KeyCode::Up | KeyCode::Char('k') if !self.settings_editor.is_editing() => {
                    if self.settings_editor.current_field() > 0 {
                        self.settings_editor.move_up();
                    }
                    return Ok(());
                }
                KeyCode::Down | KeyCode::Char('j') if !self.settings_editor.is_editing() => {
                    if self.settings_editor.current_field() < 6 {
                        self.settings_editor.move_down();
                    }
                    return Ok(());
                }
                // Global navigation - ONLY when NOT editing: enter tab-bar nav mode.
                KeyCode::Tab | KeyCode::Right | KeyCode::Left | KeyCode::BackTab
                    if !self.settings_editor.is_editing() =>
                {
                    self.enter_tab_navigation();
                    return Ok(());
                }
                KeyCode::Char('q') | KeyCode::Char('Q') if !self.settings_editor.is_editing() => {
                    self.input_mode = InputMode::QuitConfirmation;
                    self.quit_selection = QuitSelection::No;
                    return Ok(());
                }
                // Delegate all other keys to settings_editor
                _ => {
                    use crate::tui::SettingsAction;
                    let action = self.settings_editor.handle_input(event);
                    match action {
                        SettingsAction::RequestSave => {
                            // TODO: Implement password prompt and save logic
                            self.settings_editor.set_success(
                                "Guardado deshabilitado temporalmente - requiere password prompt"
                                    .to_string(),
                            );
                        }
                        SettingsAction::CancelChanges => {
                            // Reload from current config (would need to store config reference)
                            self.add_log(LogLevel::Info, "Cambios cancelados");
                        }
                        SettingsAction::None => {}
                    }
                    return Ok(());
                }
            }
        }

        // Handle Docs tab navigation specially
        if self.current_tab == Tab::Docs {
            match event.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.docs_viewing_content {
                        self.docs_scroll_offset = self.docs_scroll_offset.saturating_sub(1);
                    } else if self.docs_selected_section > 0 {
                        self.docs_selected_section -= 1;
                    }
                    return Ok(());
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.docs_viewing_content {
                        // Line counts for each section (approximate, allows some scrolling past end)
                        const DOC_LINE_COUNTS: [u16; 5] = [38, 37, 40, 35, 38];
                        let max_scroll = DOC_LINE_COUNTS
                            .get(self.docs_selected_section)
                            .copied()
                            .unwrap_or(30)
                            .saturating_sub(10); // Stop ~10 lines before end so content stays visible
                        if self.docs_scroll_offset < max_scroll {
                            self.docs_scroll_offset = self.docs_scroll_offset.saturating_add(1);
                        }
                    } else if self.docs_selected_section < 4 {
                        // 5 sections (0-4)
                        self.docs_selected_section += 1;
                    }
                    return Ok(());
                }
                KeyCode::Enter => {
                    if !self.docs_viewing_content {
                        self.docs_viewing_content = true;
                        self.docs_scroll_offset = 0;
                    }
                    return Ok(());
                }
                KeyCode::Backspace | KeyCode::Esc => {
                    if self.docs_viewing_content {
                        self.docs_viewing_content = false;
                        self.docs_scroll_offset = 0;
                    }
                    return Ok(());
                }
                // Tab-bar navigation only when NOT viewing a document's content.
                KeyCode::Left | KeyCode::BackTab if !self.docs_viewing_content => {
                    self.enter_tab_navigation();
                    return Ok(());
                }
                KeyCode::Right | KeyCode::Tab => {
                    if !self.docs_viewing_content {
                        self.enter_tab_navigation();
                    }
                    // When viewing content, arrows do nothing.
                    return Ok(());
                }
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.input_mode = InputMode::QuitConfirmation;
                    self.quit_selection = QuitSelection::No;
                    return Ok(());
                }
                KeyCode::Char('1') => {
                    self.current_tab = Tab::Dashboard;
                    return Ok(());
                }
                KeyCode::Char('2') => {
                    self.current_tab = Tab::Orders;
                    return Ok(());
                }
                KeyCode::Char('3') => {
                    self.current_tab = Tab::Markets;
                    return Ok(());
                }
                KeyCode::Char('4') => {
                    self.current_tab = Tab::MarketDetail;
                    return Ok(());
                }
                KeyCode::Char('5') => {
                    self.current_tab = Tab::Logs;
                    return Ok(());
                }
                KeyCode::Char('6') => {
                    self.current_tab = Tab::Docs;
                    return Ok(());
                }
                KeyCode::Char('c') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.should_quit = true;
                    return Ok(());
                }
                _ => return Ok(()),
            }
        }

        match event.code {
            // Enter command mode
            KeyCode::Char(':') | KeyCode::Char('/') => {
                self.input_mode = InputMode::Command;
                self.command_input = "/".to_string();
            }

            // Quick search
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.input_mode = InputMode::Command;
                self.command_input = "/search ".to_string();
                self.current_tab = Tab::Markets;
            }

            // Quick trending
            KeyCode::Char('t') | KeyCode::Char('T') => {
                self.load_trending_markets().await;
            }

            // Market navigation (when in Markets or MarketDetail tab)
            KeyCode::Up | KeyCode::Char('k') => {
                if self.current_tab == Tab::Markets && self.selected_market_index > 0 {
                    self.selected_market_index -= 1;
                } else if self.current_tab == Tab::MarketDetail
                    && self.selected_watched_market_index > 0
                {
                    self.selected_watched_market_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.current_tab == Tab::Markets
                    && self.selected_market_index < self.available_markets.len().saturating_sub(1)
                {
                    self.selected_market_index += 1;
                } else if self.current_tab == Tab::MarketDetail
                    && self.selected_watched_market_index
                        < self.watched_markets_info.len().saturating_sub(1)
                {
                    self.selected_watched_market_index += 1;
                }
            }
            KeyCode::Enter => {
                if self.current_tab == Tab::Markets && !self.available_markets.is_empty() {
                    let index = (self.selected_market_index + 1).to_string();
                    self.join_market(&index).await;
                }
            }
            // Leave market - show confirmation modal (Delete or Backspace in MarketDetail tab)
            KeyCode::Delete | KeyCode::Backspace => {
                if self.current_tab == Tab::MarketDetail && !self.watched_markets_info.is_empty() {
                    self.input_mode = InputMode::LeaveMarketConfirmation;
                    self.leave_selection = LeaveSelection::No;
                }
            }

            // Quit - show confirmation modal
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.input_mode = InputMode::QuitConfirmation;
                self.quit_selection = QuitSelection::No; // Default to No
            }

            // Tab navigation: enter tab-bar nav mode (arrows highlight, Enter activates).
            KeyCode::Tab | KeyCode::Right | KeyCode::BackTab | KeyCode::Left => {
                self.enter_tab_navigation();
            }

            // Numeric tab selection
            KeyCode::Char('1') => self.current_tab = Tab::Dashboard,
            KeyCode::Char('2') => self.current_tab = Tab::Orders,
            KeyCode::Char('3') => self.current_tab = Tab::Markets,
            KeyCode::Char('4') => self.current_tab = Tab::MarketDetail,
            KeyCode::Char('5') => self.current_tab = Tab::Logs,
            KeyCode::Char('6') => self.current_tab = Tab::Docs,
            KeyCode::Char('7') => self.current_tab = Tab::AiChat,
            KeyCode::Char('8') => self.current_tab = Tab::Settings,

            // Pause/Resume
            KeyCode::Char('p') | KeyCode::Char('P') => {
                self.execution_engine.pause().await;
                self.is_paused = true;
                self.add_log(LogLevel::Warning, "Bot PAUSED - trading disabled");
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.execution_engine.resume().await;
                self.is_paused = false;
                self.add_log(LogLevel::Success, "Bot RESUMED - trading enabled");
            }

            // Panic mode
            KeyCode::Char('!') => {
                self.add_log(LogLevel::Error, "🚨 PANIC MODE ACTIVATED");
                match self.execution_engine.cancel_all_orders().await {
                    Ok(count) => {
                        self.add_log(LogLevel::Error, &format!("Cancelled {} orders", count));
                        self.add_log(LogLevel::Error, "Bot is now PAUSED");
                    }
                    Err(e) => {
                        self.add_log(LogLevel::Error, &format!("Panic error: {}", e));
                    }
                }
                self.is_paused = true;
            }

            // Export
            KeyCode::Char('e') | KeyCode::Char('E') => {
                self.add_log(LogLevel::Info, "Export feature coming soon...");
            }

            // Help
            KeyCode::Char('h') | KeyCode::Char('H') => {
                self.add_log(LogLevel::Info, "─── Keyboard Shortcuts ───");
                self.add_log(LogLevel::Info, ":        : Enter command mode");
                self.add_log(LogLevel::Info, "S        : Quick search markets");
                self.add_log(LogLevel::Info, "T        : Load trending markets");
                self.add_log(LogLevel::Info, "Tab/←/→  : Highlight tabs (Enter to open)");
                self.add_log(LogLevel::Info, "↑/↓      : Navigate markets list");
                self.add_log(LogLevel::Info, "Enter    : Join selected market");
                self.add_log(LogLevel::Info, "P        : Pause bot");
                self.add_log(LogLevel::Info, "R        : Resume bot");
                self.add_log(LogLevel::Info, "!        : PANIC mode");
                self.add_log(LogLevel::Info, "Q        : Quit");
            }

            // Ctrl+C
            KeyCode::Char('c') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }

            _ => {}
        }

        Ok(())
    }

    fn handle_quit_confirmation(&mut self, event: KeyEvent) -> Result<()> {
        match event.code {
            // Toggle selection with Left/Right or Tab
            KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => {
                self.quit_selection = match self.quit_selection {
                    QuitSelection::No => QuitSelection::Yes,
                    QuitSelection::Yes => QuitSelection::No,
                };
            }
            // Confirm selection with Enter
            KeyCode::Enter => {
                if self.quit_selection == QuitSelection::Yes {
                    self.should_quit = true;
                }
                self.input_mode = InputMode::Normal;
            }
            // Cancel with Escape
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_leave_confirmation(&mut self, event: KeyEvent) -> Result<()> {
        match event.code {
            // Toggle selection with Left/Right or Tab
            KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => {
                self.leave_selection = match self.leave_selection {
                    LeaveSelection::No => LeaveSelection::Yes,
                    LeaveSelection::Yes => LeaveSelection::No,
                };
            }
            // Confirm selection with Enter
            KeyCode::Enter => {
                if self.leave_selection == LeaveSelection::Yes {
                    // Get the market to leave
                    if let Some(market) = self
                        .watched_markets_info
                        .get(self.selected_watched_market_index)
                    {
                        let market_id = market.id.clone();
                        self.leave_market(&market_id).await;
                        // Adjust index if needed
                        if self.selected_watched_market_index > 0
                            && self.selected_watched_market_index >= self.watched_markets_info.len()
                        {
                            self.selected_watched_market_index =
                                self.watched_markets_info.len().saturating_sub(1);
                        }
                    }
                }
                self.input_mode = InputMode::Normal;
            }
            // Cancel with Escape
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            _ => {}
        }
        Ok(())
    }
}
