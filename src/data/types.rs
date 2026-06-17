//! Shared domain types used across modules.
//!
//! Cross-module data structures live here so that no feature module owns a type another
//! module also needs. Feature-local types stay in their own module.

// =========================================================================================================
// Imports
// =========================================================================================================

use serde::{Deserialize, Serialize};

// =========================================================================================================
// Bot state
// =========================================================================================================

/// Represents the current state of the trading bot.
#[derive(Debug, Clone, Default)]
pub struct BotState {
    pub is_paused: bool,
    pub last_order_id: Option<String>,
    pub monitored_markets: Vec<String>,
}

// =========================================================================================================
// Portfolio & orders
// =========================================================================================================

/// Portfolio information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portfolio {
    pub usdc_balance: f64,
    pub total_value: f64,
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
}

/// Order information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderInfo {
    pub order_id: String,
    pub market_id: String,
    pub side: String,
    pub price: f64,
    pub size: f64,
    pub filled_size: f64,
    pub status: String,
    pub created_at: i64,
}

// =========================================================================================================
// Spike detection signals
// =========================================================================================================

/// Volume velocity spike event.
#[derive(Debug, Clone)]
pub struct VolumeVelocityEvent {
    pub market_id: String,
    pub velocity: f64,
    pub volume_delta: f64,
    pub time_delta: f64,
    pub timestamp: i64,
}

/// Order book imbalance data.
#[derive(Debug, Clone)]
pub struct OrderBookImbalance {
    pub market_id: String,
    // (V_bids - V_asks) / (V_bids + V_asks)
    pub obi: f64,
    pub bids_volume: f64,
    pub asks_volume: f64,
    pub timestamp: i64,
}
