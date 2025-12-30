use serde::{Deserialize, Serialize};

/// Represents the current state of the trading bot
#[derive(Debug, Clone)]
pub struct BotState {
    pub is_paused: bool,
    pub last_order_id: Option<String>,
    pub monitored_markets: Vec<String>,
}

impl Default for BotState {
    fn default() -> Self {
        Self {
            is_paused: false,
            last_order_id: None,
            monitored_markets: Vec::new(),
        }
    }
}

/// Portfolio information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portfolio {
    pub usdc_balance: f64,
    pub total_value: f64,
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
}

/// Order information
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

/// Market information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketInfo {
    pub market_id: String,
    pub question: String,
    pub active: bool,
}

/// Volume velocity spike event
#[derive(Debug, Clone)]
pub struct VolumeVelocityEvent {
    pub market_id: String,
    pub velocity: f64,
    pub volume_delta: f64,
    pub time_delta: f64,
    pub timestamp: i64,
}

/// Order book imbalance data
#[derive(Debug, Clone)]
pub struct OrderBookImbalance {
    pub market_id: String,
    pub obi: f64, // (V_bids - V_asks) / (V_bids + V_asks)
    pub bids_volume: f64,
    pub asks_volume: f64,
    pub timestamp: i64,
}
