//! AI integration module for market analysis

pub mod analyzer;
pub mod chatbot;
pub mod client;
pub mod personality;

pub use analyzer::{AiRecommendation, MarketAnalyzer, TradeAction};
pub use chatbot::{AiChatbot, ChatbotResponse, PendingConfirmation};
pub use client::{AiResponse, GeminiClient};
pub use personality::{AiPersonality, PersonalityTrait};
