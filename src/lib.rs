//! Polymarket Bot Summer — public library surface.
//!
//! The crate is organized into domains. Each domain is a directory module with a `mod.rs`
//! that wires up its submodules and re-exports their public items:
//!
//! - `config`  — `SecureConfig` model and encrypted persistence
//! - `trading` — CLOB auth, order execution, market data, spike detection
//! - `data`    — SQLite persistence and shared domain types
//! - `ai`      — Gemini client, market analyzer, chatbot, personalities
//! - `tui`     — ratatui terminal UI and its screens

// =========================================================================================================
// Domains
// =========================================================================================================

pub mod ai;
pub mod config;
pub mod data;
pub mod trading;
pub mod tui;

// =========================================================================================================
// Re-exports
// =========================================================================================================

pub use ai::{
    AiChatbot, AiPersonality, AiRecommendation, AiResponse, ChatbotResponse, GeminiClient,
    MarketAnalyzer, PendingConfirmation, PersonalityTrait, TradeAction,
};
pub use config::{
    change_password, decrypt_config, encrypt_config, load_config, save_config, SecureConfig,
};
pub use data::{init_database, DbPool};
pub use trading::{authenticate, ExecutionEngine, MarketService, SpikeDetector};
pub use tui::{run_tui, ChatState, ConfigWizard, PasswordPrompt};
