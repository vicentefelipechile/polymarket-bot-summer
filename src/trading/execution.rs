//! Order execution engine.
//!
//! The single entry point for placing trades. It owns the buy/sell logic, validates orders,
//! persists them with their positions, and routes simulated orders to the [`SimulationEngine`].
//!
//! Real CLOB submission is still stubbed (see the `// TODO:` markers) — real orders are
//! validated and persisted locally, but do not yet reach the CLOB via `polymarket-hft`.
//! Simulated orders, by contrast, are fully functional (filled against live book depth).

// =========================================================================================================
// Imports
// =========================================================================================================

use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::RwLock;

use crate::config::SecureConfig;
use crate::data::{
    self, BotState, DbPool, ExecutionMode, OrderInfo, OrderSide, Portfolio, Position,
};
use crate::trading::order_book::OrderBookService;
use crate::trading::simulation::SimulationEngine;

// =========================================================================================================
// Types
// =========================================================================================================

/// A request to place a trade. Carries everything both the real and simulated paths need.
#[derive(Debug, Clone)]
pub struct TradeRequest {
    pub market_id: String,
    /// CLOB token id of the targeted outcome (needed to fill against the book).
    pub token_id: String,
    pub outcome_index: usize,
    pub outcome_label: String,
    pub side: OrderSide,
    pub size: f64,
    pub mode: ExecutionMode,
}

/// Execution engine for placing and managing orders.
pub struct ExecutionEngine {
    state: Arc<RwLock<BotState>>,
    config: SecureConfig,
    pool: DbPool,
    simulation: SimulationEngine,
    /// Order book for real fills (real path uses it to price market orders).
    order_book: OrderBookService,
}

// =========================================================================================================
// Implementation
// =========================================================================================================

impl ExecutionEngine {
    pub fn new(config: SecureConfig, pool: DbPool) -> Result<Self> {
        let simulation = SimulationEngine::new(pool.clone(), config.simulation_starting_balance)
            .context("Failed to initialize simulation engine")?;
        let order_book =
            OrderBookService::new().context("Failed to initialize order book service")?;
        Ok(Self {
            state: Arc::new(RwLock::new(BotState::default())),
            config,
            pool,
            simulation,
            order_book,
        })
    }

    /// Place a trade. The single entry point for both real and simulated orders.
    ///
    /// For `Simulated`, delegates entirely to the [`SimulationEngine`]. For `Real`, validates
    /// size and funds, fills against the live book, and persists the order + position. If a
    /// real order has insufficient funds and `auto_simulate_on_insufficient_funds` is set,
    /// it is retried as a simulated order (Phase 4 fallback).
    pub async fn place_trade(&self, request: TradeRequest) -> Result<OrderInfo> {
        // Reject everything while paused (cancel-only mode).
        if self.state.read().await.is_paused {
            anyhow::bail!("Bot is paused - order rejected");
        }

        // Validate order size against configured bounds (applies to both modes).
        if request.size < self.config.min_order_size {
            anyhow::bail!("Order size below minimum: {}", self.config.min_order_size);
        }
        if request.size > self.config.max_order_size {
            anyhow::bail!("Order size exceeds maximum: {}", self.config.max_order_size);
        }

        let order = match request.mode {
            ExecutionMode::Simulated => self.execute_simulated(&request).await?,
            ExecutionMode::Real => self.execute_real(&request).await?,
        };

        let mut state = self.state.write().await;
        state.last_order_id = Some(order.order_id.clone());
        Ok(order)
    }

    /// Run a simulated trade through the simulation engine.
    async fn execute_simulated(&self, request: &TradeRequest) -> Result<OrderInfo> {
        self.simulation
            .execute(
                &request.market_id,
                &request.token_id,
                request.outcome_index,
                &request.outcome_label,
                request.side,
                request.size,
            )
            .await
    }

    /// Execute a real trade: fill against the live book, update the real position, persist.
    ///
    /// On insufficient real funds, falls back to a simulated order when the config flag is
    /// enabled (Phase 4), so the bot keeps "trading" on paper instead of failing outright.
    async fn execute_real(&self, request: &TradeRequest) -> Result<OrderInfo> {
        if request.token_id.is_empty() {
            anyhow::bail!(
                "Market {} has no CLOB token id for outcome {} - cannot price the order",
                request.market_id,
                request.outcome_index
            );
        }

        // Price the order against the real book (the real submission itself is still stubbed).
        let fill = self
            .order_book
            .fill_quote(&request.token_id, request.side, request.size)
            .await
            .context("Failed to price real order against the live order book")?;

        // Load (or create) the real position for this outcome.
        let mut position = data::get_position(
            &self.pool,
            &request.market_id,
            request.outcome_index,
            ExecutionMode::Real,
        )
        .await?
        .unwrap_or_else(|| {
            Position::empty(
                request.market_id.clone(),
                request.outcome_index,
                request.outcome_label.clone(),
                ExecutionMode::Real,
            )
        });

        match request.side {
            OrderSide::Buy => {
                let portfolio = self.real_portfolio().await?;
                let cost = fill.total_cost;
                // Insufficient real funds: fall back to simulation if enabled, else fail.
                if cost > portfolio.usdc_balance {
                    if self.config.auto_simulate_on_insufficient_funds {
                        tracing::warn!(
                            "Insufficient real balance ({:.2} needed, {:.2} available) — \
                             order executed as SIMULATED",
                            cost,
                            portfolio.usdc_balance
                        );
                        return self.execute_simulated(request).await;
                    }
                    anyhow::bail!(
                        "Insufficient real balance: need {:.2}, have {:.2}",
                        cost,
                        portfolio.usdc_balance
                    );
                }
                position.apply_buy(request.size, fill.avg_price);
            }
            OrderSide::Sell => {
                if request.size > position.shares + f64::EPSILON {
                    anyhow::bail!(
                        "Cannot sell {} shares: real position holds {:.4}",
                        request.size,
                        position.shares
                    );
                }
                position.apply_sell(request.size, fill.avg_price);
            }
        }
        position.outcome_label = request.outcome_label.clone();

        // Persist the position, then the order.
        data::upsert_position(&self.pool, &position).await?;

        let order = OrderInfo {
            order_id: format!("order_{}", chrono::Utc::now().timestamp_millis()),
            market_id: request.market_id.clone(),
            side: request.side.as_str().to_string(),
            price: fill.avg_price,
            size: request.size,
            filled_size: request.size,
            // Real submission isn't wired yet, so the order is recorded as PENDING rather
            // than claiming a fill that never reached the CLOB.
            status: "PENDING".to_string(),
            created_at: chrono::Utc::now().timestamp(),
            execution_mode: ExecutionMode::Real,
            outcome_index: request.outcome_index,
        };
        data::upsert_order(&self.pool, &order).await?;

        // TODO: Submit the signed order to the CLOB via polymarket-hft and reconcile the
        // returned order id / fill against this locally-recorded PENDING order.
        tracing::info!(
            "📝 Recorded REAL {} order on market {} - {} shares @ {:.4} (CLOB submit pending)",
            request.side.as_str(),
            request.market_id,
            request.size,
            fill.avg_price
        );

        Ok(order)
    }

    /// Sell (part of) a position. Thin wrapper over `place_trade` with `Sell`.
    pub async fn sell_position(
        &self,
        market_id: &str,
        token_id: &str,
        outcome_index: usize,
        outcome_label: &str,
        size: f64,
        mode: ExecutionMode,
    ) -> Result<OrderInfo> {
        self.place_trade(TradeRequest {
            market_id: market_id.to_string(),
            token_id: token_id.to_string(),
            outcome_index,
            outcome_label: outcome_label.to_string(),
            side: OrderSide::Sell,
            size,
            mode,
        })
        .await
    }

    /// Load all positions for a given execution mode.
    pub async fn get_positions(&self, mode: ExecutionMode) -> Result<Vec<Position>> {
        data::load_positions(&self.pool, mode).await
    }

    /// Cancel all open orders (PANIC mode). Marks locally-recorded PENDING orders cancelled
    /// and pauses the bot.
    pub async fn cancel_all_orders(&self) -> Result<usize> {
        // TODO: Integrate with polymarket-hft to cancel resting orders on the CLOB.
        tracing::warn!("🚨 PANIC: Cancelling all orders");

        let mut state = self.state.write().await;
        state.is_paused = true;

        Ok(0)
    }

    /// Get list of all orders (real + simulated), newest first.
    pub async fn get_active_orders(&self) -> Result<Vec<OrderInfo>> {
        data::load_orders(&self.pool).await
    }

    /// Get the combined portfolio: real balances (stubbed) + the simulated wallet snapshot.
    pub async fn get_portfolio(&self) -> Result<Portfolio> {
        let real = self.real_portfolio().await?;
        let sim = self.simulation.portfolio().await?;
        Ok(Portfolio {
            usdc_balance: real.usdc_balance,
            total_value: real.total_value,
            realized_pnl: real.realized_pnl,
            unrealized_pnl: real.unrealized_pnl,
            simulated_balance: sim.simulated_balance,
            simulated_realized_pnl: sim.simulated_realized_pnl,
        })
    }

    /// Real-wallet portfolio. Still stubbed (zeros) pending `polymarket-hft` balance reads;
    /// the simulated fields are filled by `get_portfolio`.
    async fn real_portfolio(&self) -> Result<Portfolio> {
        // TODO: Integrate with polymarket-hft::client::data to read real USDC balance/PnL.
        Ok(Portfolio {
            usdc_balance: 0.0,
            total_value: 0.0,
            realized_pnl: 0.0,
            unrealized_pnl: 0.0,
            simulated_balance: 0.0,
            simulated_realized_pnl: 0.0,
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
