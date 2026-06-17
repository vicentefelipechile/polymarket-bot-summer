//! Paper-trading (simulation) engine.
//!
//! Fills orders against the **real** CLOB order book (via [`OrderBookService`]) but settles
//! them against a **separate virtual wallet** — no funds ever move. Everything it produces
//! is tagged [`ExecutionMode::Simulated`] so the UI can show simulated orders/positions
//! beside real ones with a `SIMULATED` label.
//!
//! A configurable per-fill slippage and fee make paper fills slightly worse than the raw
//! book VWAP, so simulated results stay conservative rather than flattering.

// =========================================================================================================
// Imports
// =========================================================================================================

use anyhow::{Context, Result};

use crate::data::{self, DbPool, ExecutionMode, OrderInfo, OrderSide, Portfolio, Position};
use crate::trading::order_book::OrderBookService;

// =========================================================================================================
// Constants
// =========================================================================================================

/// Extra adverse price applied to every simulated fill, as a fraction of price. Models the
/// slippage a taker eats beyond the visible VWAP (latency, partial-fill churn).
const SIMULATED_SLIPPAGE: f64 = 0.001;

/// Flat taker fee applied to the notional of every simulated fill, as a fraction.
const SIMULATED_FEE: f64 = 0.0;

// =========================================================================================================
// Types
// =========================================================================================================

/// Engine that executes simulated trades against real book depth and a virtual wallet.
pub struct SimulationEngine {
    pool: DbPool,
    order_book: OrderBookService,
    starting_balance: f64,
}

// =========================================================================================================
// Implementation
// =========================================================================================================

impl SimulationEngine {
    /// Build the engine. The virtual wallet is seeded lazily with `starting_balance` the
    /// first time a trade runs (or balance is queried).
    pub fn new(pool: DbPool, starting_balance: f64) -> Result<Self> {
        let order_book = OrderBookService::new()?;
        Ok(Self {
            pool,
            order_book,
            starting_balance,
        })
    }

    /// Execute a simulated trade: fill against the real book, debit/credit the virtual
    /// wallet, update the simulated position, and persist a `SIMULATED` order row.
    ///
    /// Returns the persisted order (status `FILLED`). Errors on insufficient virtual funds
    /// (buy) or insufficient held shares (sell) — never silently partial-fills.
    pub async fn execute(
        &self,
        market_id: &str,
        token_id: &str,
        outcome_index: usize,
        outcome_label: &str,
        side: OrderSide,
        size: f64,
    ) -> Result<OrderInfo> {
        // Seed the wallet on first use and read the current balance.
        let balance = data::ensure_simulation_account(&self.pool, self.starting_balance)
            .await
            .context("Failed to access simulation account")?;
        let (_, realized_pnl) = data::get_simulation_account(&self.pool)
            .await?
            .unwrap_or((balance, 0.0));

        // Fill against real depth, then apply simulated slippage + fee so paper results are
        // conservative. Buys pay slightly more per share; sells receive slightly less.
        let fill = self
            .order_book
            .fill_quote(token_id, side, size)
            .await
            .context("Simulated fill against live order book failed")?;

        let exec_price = match side {
            OrderSide::Buy => fill.avg_price * (1.0 + SIMULATED_SLIPPAGE),
            OrderSide::Sell => fill.avg_price * (1.0 - SIMULATED_SLIPPAGE),
        };
        let notional = exec_price * size;
        let fee = notional * SIMULATED_FEE;

        // Load (or create) the simulated position for this outcome.
        let mut position = data::get_position(
            &self.pool,
            market_id,
            outcome_index,
            ExecutionMode::Simulated,
        )
        .await?
        .unwrap_or_else(|| {
            Position::empty(
                market_id.to_string(),
                outcome_index,
                outcome_label.to_string(),
                ExecutionMode::Simulated,
            )
        });

        // Apply the trade to the wallet and the position.
        let (new_balance, new_realized_pnl) = match side {
            OrderSide::Buy => {
                let total_debit = notional + fee;
                if total_debit > balance {
                    anyhow::bail!(
                        "Insufficient simulated balance: need {:.2}, have {:.2}",
                        total_debit,
                        balance
                    );
                }
                position.apply_buy(size, exec_price);
                (balance - total_debit, realized_pnl)
            }
            OrderSide::Sell => {
                if size > position.shares + f64::EPSILON {
                    anyhow::bail!(
                        "Cannot sell {} shares: simulated position holds {:.4}",
                        size,
                        position.shares
                    );
                }
                let pnl = position.apply_sell(size, exec_price);
                // Credit proceeds (net of fee); realized PnL tracks the booked gain/loss.
                (balance + notional - fee, realized_pnl + pnl)
            }
        };
        // Keep the label fresh in case it changed since the position was opened.
        position.outcome_label = outcome_label.to_string();

        // Persist position, wallet, then the order. Order is last so a failed write doesn't
        // leave a phantom FILLED order without the matching position/balance change.
        data::upsert_position(&self.pool, &position).await?;
        data::update_simulation_account(&self.pool, new_balance, new_realized_pnl).await?;

        let order = OrderInfo {
            order_id: format!("sim_{}", chrono::Utc::now().timestamp_millis()),
            market_id: market_id.to_string(),
            side: side.as_str().to_string(),
            price: exec_price,
            size,
            filled_size: size,
            status: "FILLED".to_string(),
            created_at: chrono::Utc::now().timestamp(),
            execution_mode: ExecutionMode::Simulated,
            outcome_index,
        };
        data::upsert_order(&self.pool, &order).await?;

        tracing::info!(
            "🧪 SIMULATED {} {:.4} shares of '{}' @ {:.4} (notional {:.2}, balance {:.2})",
            side.as_str(),
            size,
            outcome_label,
            exec_price,
            notional,
            new_balance
        );

        Ok(order)
    }

    /// Build a portfolio snapshot for the simulated wallet: virtual balance + realized PnL.
    /// Unrealized PnL is left to the caller (it needs live prices per open position).
    pub async fn portfolio(&self) -> Result<Portfolio> {
        let balance = data::ensure_simulation_account(&self.pool, self.starting_balance).await?;
        let (_, realized_pnl) = data::get_simulation_account(&self.pool)
            .await?
            .unwrap_or((balance, 0.0));

        Ok(Portfolio {
            usdc_balance: 0.0,
            total_value: 0.0,
            realized_pnl: 0.0,
            unrealized_pnl: 0.0,
            simulated_balance: balance,
            simulated_realized_pnl: realized_pnl,
        })
    }
}
