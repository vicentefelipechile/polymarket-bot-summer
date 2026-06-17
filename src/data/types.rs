//! Shared domain types used across modules.
//!
//! Cross-module data structures live here so that no feature module owns a type another
//! module also needs. Feature-local types stay in their own module.

// =========================================================================================================
// Imports
// =========================================================================================================

use std::fmt;

use serde::{Deserialize, Serialize};

// =========================================================================================================
// Execution mode, order side & type
// =========================================================================================================

/// Whether an order/position is backed by real funds or the paper-trading simulator.
///
/// `Display` and `as_str` render `Simulated` as the user-facing `"SIMULATED"` label and
/// `Real` as `"REAL"`; both round-trip through `from_str` for DB persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExecutionMode {
    #[default]
    Real,
    Simulated,
}

impl ExecutionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Real => "REAL",
            Self::Simulated => "SIMULATED",
        }
    }

    /// Parse from the persisted string. Unknown values fall back to `Real` so a corrupt
    /// row never silently becomes a simulated position the user didn't create.
    pub fn parse_str(value: &str) -> Self {
        match value.trim().to_ascii_uppercase().as_str() {
            "SIMULATED" => Self::Simulated,
            _ => Self::Real,
        }
    }
}

impl fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Direction of an order: buying or selling outcome shares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl OrderSide {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Buy => "BUY",
            Self::Sell => "SELL",
        }
    }

    /// Parse from a persisted/UI string. Unknown values fall back to `Buy`.
    pub fn parse_str(value: &str) -> Self {
        match value.trim().to_ascii_uppercase().as_str() {
            "SELL" => Self::Sell,
            _ => Self::Buy,
        }
    }
}

impl fmt::Display for OrderSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Order type: a marketable order filled against the book, or a resting limit order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OrderType {
    #[default]
    Market,
    Limit,
}

impl OrderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Market => "MARKET",
            Self::Limit => "LIMIT",
        }
    }

    pub fn parse_str(value: &str) -> Self {
        match value.trim().to_ascii_uppercase().as_str() {
            "LIMIT" => Self::Limit,
            _ => Self::Market,
        }
    }
}

impl fmt::Display for OrderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

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
///
/// Real and simulated balances are tracked separately so the dashboard can show both
/// without one masking the other; `total_value` / PnL fields describe the same `mode`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portfolio {
    pub usdc_balance: f64,
    pub total_value: f64,
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
    /// Virtual balance of the separate simulation wallet (paper trading).
    pub simulated_balance: f64,
    /// Realized PnL accumulated by simulated positions.
    pub simulated_realized_pnl: f64,
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
    /// Real vs simulated; drives the `SIMULATED` pill in the orders panel.
    pub execution_mode: ExecutionMode,
    /// Index of the outcome this order targets within the market's outcome list.
    pub outcome_index: usize,
}

// =========================================================================================================
// Positions
// =========================================================================================================

/// A holding of shares in a single market outcome, real or simulated.
///
/// `avg_price` is the volume-weighted average cost of the open shares; `realized_pnl`
/// accumulates as shares are sold. Real and simulated holdings of the same outcome are
/// distinct positions, keyed by `(market_id, outcome_index, mode)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub market_id: String,
    pub outcome_index: usize,
    pub outcome_label: String,
    pub shares: f64,
    pub avg_price: f64,
    pub realized_pnl: f64,
    pub mode: ExecutionMode,
}

impl Position {
    /// A fresh, empty position for an outcome.
    pub fn empty(
        market_id: String,
        outcome_index: usize,
        outcome_label: String,
        mode: ExecutionMode,
    ) -> Self {
        Self {
            market_id,
            outcome_index,
            outcome_label,
            shares: 0.0,
            avg_price: 0.0,
            realized_pnl: 0.0,
            mode,
        }
    }

    /// Apply a buy fill: increase shares and roll `avg_price` into the new volume-weighted
    /// average cost basis. Returns nothing — `realized_pnl` is unaffected by buys.
    pub fn apply_buy(&mut self, shares: f64, price: f64) {
        let prior_cost = self.shares * self.avg_price;
        let added_cost = shares * price;
        let new_shares = self.shares + shares;
        self.avg_price = if new_shares > 0.0 {
            (prior_cost + added_cost) / new_shares
        } else {
            0.0
        };
        self.shares = new_shares;
    }

    /// Apply a sell fill: reduce shares and bank realized PnL against the average cost
    /// basis. `avg_price` is unchanged by a sell (remaining shares keep their basis).
    /// Returns the realized PnL of this sell. Selling is clamped to the held shares.
    pub fn apply_sell(&mut self, shares: f64, price: f64) -> f64 {
        let sold = shares.min(self.shares);
        let pnl = (price - self.avg_price) * sold;
        self.shares -= sold;
        self.realized_pnl += pnl;
        // Once flat, reset the basis so a later re-entry starts clean.
        if self.shares <= f64::EPSILON {
            self.shares = 0.0;
            self.avg_price = 0.0;
        }
        pnl
    }
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

// =========================================================================================================
// Tests
// =========================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn new_position() -> Position {
        Position::empty("mkt".to_string(), 0, "Yes".to_string(), ExecutionMode::Real)
    }

    #[test]
    fn execution_mode_roundtrips() {
        for mode in [ExecutionMode::Real, ExecutionMode::Simulated] {
            assert_eq!(ExecutionMode::parse_str(mode.as_str()), mode);
        }
        // Display matches as_str (renders SIMULATED).
        assert_eq!(ExecutionMode::Simulated.to_string(), "SIMULATED");
        // Unknown / corrupt values fall back to Real, never silently simulated.
        assert_eq!(ExecutionMode::parse_str("garbage"), ExecutionMode::Real);
    }

    #[test]
    fn order_side_roundtrips() {
        assert_eq!(OrderSide::parse_str("BUY"), OrderSide::Buy);
        assert_eq!(OrderSide::parse_str("sell"), OrderSide::Sell);
        assert_eq!(OrderSide::parse_str("???"), OrderSide::Buy);
    }

    #[test]
    fn buy_sets_weighted_average_cost() {
        let mut p = new_position();
        p.apply_buy(10.0, 0.40);
        assert!((p.shares - 10.0).abs() < 1e-9);
        assert!((p.avg_price - 0.40).abs() < 1e-9);

        // Buying more rolls into a volume-weighted average: (10*0.40 + 30*0.60)/40 = 0.55.
        p.apply_buy(30.0, 0.60);
        assert!((p.shares - 40.0).abs() < 1e-9);
        assert!((p.avg_price - 0.55).abs() < 1e-9);
        // Buys never realize PnL.
        assert!(p.realized_pnl.abs() < 1e-9);
    }

    #[test]
    fn partial_sell_books_realized_pnl_and_keeps_basis() {
        let mut p = new_position();
        p.apply_buy(100.0, 0.50);

        // Sell 40 @ 0.60: realized = (0.60 - 0.50) * 40 = 4.0.
        let pnl = p.apply_sell(40.0, 0.60);
        assert!((pnl - 4.0).abs() < 1e-9);
        assert!((p.shares - 60.0).abs() < 1e-9);
        // Remaining shares keep the original cost basis.
        assert!((p.avg_price - 0.50).abs() < 1e-9);
        assert!((p.realized_pnl - 4.0).abs() < 1e-9);
    }

    #[test]
    fn full_sell_flattens_and_resets_basis() {
        let mut p = new_position();
        p.apply_buy(50.0, 0.20);

        // Sell everything at a loss: (0.10 - 0.20) * 50 = -5.0.
        let pnl = p.apply_sell(50.0, 0.10);
        assert!((pnl + 5.0).abs() < 1e-9);
        assert_eq!(p.shares, 0.0);
        // Once flat, the basis resets so a re-entry starts clean.
        assert_eq!(p.avg_price, 0.0);
        assert!((p.realized_pnl + 5.0).abs() < 1e-9);
    }

    #[test]
    fn sell_is_clamped_to_held_shares() {
        let mut p = new_position();
        p.apply_buy(10.0, 0.50);
        // Asking to sell more than held only sells what's there.
        let pnl = p.apply_sell(25.0, 0.60);
        assert_eq!(p.shares, 0.0);
        // Realized only on the 10 actually held: (0.60 - 0.50) * 10 = 1.0.
        assert!((pnl - 1.0).abs() < 1e-9);
    }
}
