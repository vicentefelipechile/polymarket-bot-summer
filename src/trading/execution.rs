//! Order execution engine.
//!
//! Placeholder for the real trading path: order placement, cancellation, portfolio, and
//! pause/resume state. The actual `polymarket-hft` integration is still stubbed (see the
//! `// TODO:` markers) — orders do not reach the CLOB yet.

// =========================================================================================================
// Imports
// =========================================================================================================

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;

use crate::config::SecureConfig;
use crate::data::{BotState, OrderInfo, Portfolio};

// =========================================================================================================
// Types
// =========================================================================================================

/// Execution engine for placing and managing orders.
pub struct ExecutionEngine {
    state: Arc<RwLock<BotState>>,
    config: SecureConfig,
}

// =========================================================================================================
// Implementation
// =========================================================================================================

impl ExecutionEngine {
    pub fn new(config: SecureConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(BotState::default())),
            config,
        }
    }

    /// Place a market order
    pub async fn place_order(
        &self,
        market_id: &str,
        side: &str,
        size: f64,
        price: f64,
    ) -> Result<String> {
        // Check if bot is paused
        let state = self.state.read().await;
        if state.is_paused {
            anyhow::bail!("Bot is paused - order rejected");
        }
        drop(state);

        // Validate order size
        if size < self.config.min_order_size {
            anyhow::bail!("Order size below minimum: {}", self.config.min_order_size);
        }

        if size > self.config.max_order_size {
            anyhow::bail!("Order size exceeds maximum: {}", self.config.max_order_size);
        }

        // TODO: Integrate with polymarket-hft::client::clob
        // For now, return a mock order ID
        let order_id = format!("order_{}", chrono::Utc::now().timestamp_millis());

        // Update state
        let mut state = self.state.write().await;
        state.last_order_id = Some(order_id.clone());

        tracing::info!(
            "📝 Placed {} order on market {} - Size: {} @ Price: {}",
            side,
            market_id,
            size,
            price
        );

        Ok(order_id)
    }

    /// Cancel all open orders (PANIC mode)
    pub async fn cancel_all_orders(&self) -> Result<usize> {
        // TODO: Integrate with polymarket-hft to cancel all orders
        tracing::warn!("🚨 PANIC: Cancelling all orders");

        // Pause the bot
        let mut state = self.state.write().await;
        state.is_paused = true;

        Ok(0) // Return number of cancelled orders
    }

    /// Get list of active orders
    pub async fn get_active_orders(&self) -> Result<Vec<OrderInfo>> {
        // TODO: Integrate with polymarket-hft to fetch active orders
        Ok(Vec::new())
    }

    /// Get current portfolio state
    pub async fn get_portfolio(&self) -> Result<Portfolio> {
        // TODO: Integrate with polymarket-hft::client::data
        Ok(Portfolio {
            usdc_balance: 0.0,
            total_value: 0.0,
            realized_pnl: 0.0,
            unrealized_pnl: 0.0,
        })
    }

    /// Pause the bot (cancel-only mode)
    pub async fn pause(&self) {
        let mut state = self.state.write().await;
        state.is_paused = true;
        tracing::info!("⏸️  Bot paused - entering cancel-only mode");
    }

    /// Resume normal trading
    pub async fn resume(&self) {
        let mut state = self.state.write().await;
        state.is_paused = false;
        tracing::info!("▶️  Bot resumed - trading enabled");
    }

    /// Check if bot is paused
    pub async fn is_paused(&self) -> bool {
        self.state.read().await.is_paused
    }

    /// Get the last order ID
    pub async fn get_last_order_id(&self) -> Option<String> {
        self.state.read().await.last_order_id.clone()
    }
}
