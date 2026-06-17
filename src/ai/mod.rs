//! AI integration domain: Gemini client, market analyzer, chatbot, and personalities.

// =========================================================================================================
// Submodules
// =========================================================================================================

pub mod analyzer;
pub mod chatbot;
pub mod client;
pub mod personality;

// =========================================================================================================
// Re-exports
// =========================================================================================================

pub use analyzer::{AiRecommendation, MarketAnalyzer, TradeAction};
pub use chatbot::{AiChatbot, ChatbotResponse, PendingConfirmation};
pub use client::{AiResponse, GeminiClient};
pub use personality::{AiPersonality, PersonalityTrait};
