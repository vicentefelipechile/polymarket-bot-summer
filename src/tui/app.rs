use crate::execution::ExecutionEngine;
use crate::markets::{MarketInfo, MarketService};
use crate::types::{OrderInfo, Portfolio};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::sync::Arc;
use std::time::Instant;

/// Available tabs in the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Orders,
    Markets,
    Logs,
}

impl Tab {
    pub fn next(&self) -> Self {
        match self {
            Tab::Dashboard => Tab::Orders,
            Tab::Orders => Tab::Markets,
            Tab::Markets => Tab::Logs,
            Tab::Logs => Tab::Dashboard,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Tab::Dashboard => Tab::Logs,
            Tab::Orders => Tab::Dashboard,
            Tab::Markets => Tab::Orders,
            Tab::Logs => Tab::Markets,
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Orders => "Orders",
            Tab::Markets => "Markets",
            Tab::Logs => "Logs",
        }
    }

    pub fn all() -> [Tab; 4] {
        [Tab::Dashboard, Tab::Orders, Tab::Markets, Tab::Logs]
    }
}

/// Input mode for command entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Command,
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

/// Main application state
pub struct App {
    pub execution_engine: Arc<ExecutionEngine>,
    pub market_service: MarketService,
    pub current_tab: Tab,
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

    // Markets
    pub available_markets: Vec<MarketInfo>,
    pub joined_markets: Vec<String>,
    pub market_search_query: String,
    pub selected_market_index: usize,
    pub is_loading_markets: bool,
}

impl App {
    pub fn new(execution_engine: Arc<ExecutionEngine>) -> Self {
        let mut app = Self {
            execution_engine,
            market_service: MarketService::new(),
            current_tab: Tab::Dashboard,
            should_quit: false,
            logs: Vec::new(),
            portfolio: None,
            active_orders: Vec::new(),
            is_paused: false,
            last_order_id: None,
            last_refresh: Instant::now(),
            input_mode: InputMode::Normal,
            command_input: String::new(),
            available_markets: Vec::new(),
            joined_markets: Vec::new(),
            market_search_query: String::new(),
            selected_market_index: 0,
            is_loading_markets: false,
        };

        app.add_log(LogLevel::Info, "TUI initialized successfully");
        app.add_log(LogLevel::Info, "Press ':' to enter command mode");
        app.add_log(LogLevel::Info, "Press 'S' to search markets");
        app
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
    }

    pub async fn handle_event(&mut self, event: KeyEvent) -> Result<()> {
        match self.input_mode {
            InputMode::Command => self.handle_command_input(event).await,
            InputMode::Normal => self.handle_normal_input(event).await,
        }
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
        let parts: Vec<&str> = command.trim().split_whitespace().collect();
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
                    self.leave_market(args[0]);
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
                let market = &self.available_markets[index - 1];
                let market_id = market.id.clone();
                let question = market.question.clone();

                if !self.joined_markets.contains(&market_id) {
                    self.joined_markets.push(market_id.clone());
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

        // Otherwise treat as market ID
        let market_id = market_ref.to_string();
        if !self.joined_markets.contains(&market_id) {
            self.joined_markets.push(market_id.clone());
            self.add_log(LogLevel::Success, &format!("Joined market: {}", market_id));
        } else {
            self.add_log(LogLevel::Warning, "Already monitoring this market");
        }
    }

    fn leave_market(&mut self, market_id: &str) {
        if let Some(pos) = self.joined_markets.iter().position(|m| m == market_id) {
            self.joined_markets.remove(pos);
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

            // Market navigation (when in Markets tab)
            KeyCode::Up | KeyCode::Char('k') => {
                if self.current_tab == Tab::Markets && self.selected_market_index > 0 {
                    self.selected_market_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.current_tab == Tab::Markets
                    && self.selected_market_index < self.available_markets.len().saturating_sub(1)
                {
                    self.selected_market_index += 1;
                }
            }
            KeyCode::Enter => {
                if self.current_tab == Tab::Markets && !self.available_markets.is_empty() {
                    let index = (self.selected_market_index + 1).to_string();
                    self.join_market(&index).await;
                }
            }

            // Quit
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.should_quit = true;
            }

            // Tab navigation
            KeyCode::Tab | KeyCode::Right => {
                self.current_tab = self.current_tab.next();
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.current_tab = self.current_tab.prev();
            }

            // Numeric tab selection
            KeyCode::Char('1') => self.current_tab = Tab::Dashboard,
            KeyCode::Char('2') => self.current_tab = Tab::Orders,
            KeyCode::Char('3') => self.current_tab = Tab::Markets,
            KeyCode::Char('4') => self.current_tab = Tab::Logs,

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
                self.add_log(LogLevel::Info, "Tab/←/→  : Navigate tabs");
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
}
