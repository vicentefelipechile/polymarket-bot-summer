// Core modules
pub mod ai;
pub mod clob_auth;
pub mod config;
pub mod crypto;
pub mod database;
pub mod execution;
pub mod markets;
pub mod onboarding;
pub mod spike_detection;
pub mod tui;
pub mod types;

// Re-exports
pub use ai::{
    AiChatbot, AiPersonality, AiRecommendation, AiResponse, ChatbotResponse, GeminiClient,
    MarketAnalyzer, PendingConfirmation, PersonalityTrait, TradeAction,
};
pub use clob_auth::authenticate;
pub use crypto::{
    change_password, decrypt_config, encrypt_config, load_config, save_config,
    AiPersonality as SecureAiPersonality, SecureConfig,
};
pub use database::{init_database, DbPool};
pub use execution::ExecutionEngine;
pub use markets::MarketService;
pub use onboarding::run_onboarding_checks;
pub use spike_detection::SpikeDetector;
pub use tui::{run_tui, ChatState, ConfigWizard, PasswordPrompt};
