//! Real CLOB order-book access and quote/fill simulation against live depth.
//!
//! Wraps an **unauthenticated** `clob::Client` (the `/book` endpoint needs no auth) and
//! walks the real bid/ask depth to compute the volume-weighted price a marketable order of
//! a given size would receive. Used by both the simulation engine (to fill paper orders at
//! realistic prices) and, eventually, the real execution path.

// =========================================================================================================
// Imports
// =========================================================================================================

use std::str::FromStr;

use anyhow::{Context, Result};
use polymarket_client_sdk::clob::types::request::OrderBookSummaryRequest;
use polymarket_client_sdk::clob::{Client, Config};
use polymarket_client_sdk::types::U256;
use rust_decimal::prelude::ToPrimitive;

use crate::data::OrderSide;

// =========================================================================================================
// Constants
// =========================================================================================================

const CLOB_HOST: &str = "https://clob.polymarket.com";

// =========================================================================================================
// Types
// =========================================================================================================

/// The result of filling a marketable order against the real order book.
#[derive(Debug, Clone)]
pub struct FillResult {
    /// Shares actually filled (may be less than requested if depth ran out — but we error
    /// on insufficient liquidity instead, so this equals the requested size on success).
    pub filled_shares: f64,
    /// Volume-weighted average price paid (buy) or received (sell) per share.
    pub avg_price: f64,
    /// Total USDC notional: `filled_shares * avg_price`.
    pub total_cost: f64,
}

/// Service that queries the live CLOB order book and simulates fills against it.
pub struct OrderBookService {
    client: Client,
}

// =========================================================================================================
// Implementation
// =========================================================================================================

impl OrderBookService {
    /// Build a service backed by an unauthenticated CLOB client.
    pub fn new() -> Result<Self> {
        let client = Client::new(CLOB_HOST, Config::default())
            .context("Failed to create unauthenticated CLOB client for order book")?;
        Ok(Self { client })
    }

    /// Compute the fill for a marketable order of `size` shares on `token_id`.
    ///
    /// A buy is matched against the asks (cheapest first); a sell against the bids (highest
    /// first). Walking levels in price-priority order yields the realistic VWAP a taker
    /// would pay. Errors — never silently — on an empty book or insufficient depth, and on a
    /// size below the book's `min_order_size`.
    pub async fn fill_quote(
        &self,
        token_id: &str,
        side: OrderSide,
        size: f64,
    ) -> Result<FillResult> {
        if size <= 0.0 {
            anyhow::bail!("Order size must be positive");
        }

        let token = U256::from_str(token_id)
            .with_context(|| format!("Invalid CLOB token id: '{}'", token_id))?;

        let request = OrderBookSummaryRequest::builder().token_id(token).build();
        let book = self
            .client
            .order_book(&request)
            .await
            .with_context(|| format!("Failed to fetch order book for token {}", token_id))?;

        let min_order_size = book.min_order_size.to_f64().unwrap_or(0.0);
        if size < min_order_size {
            anyhow::bail!(
                "Order size {} below market minimum {}",
                size,
                min_order_size
            );
        }

        // A buy consumes asks; a sell consumes bids. The CLOB returns each side already
        // sorted by price-priority for a taker, but we sort defensively so a level ordering
        // change upstream can't silently produce a worse-than-best VWAP.
        let mut levels: Vec<(f64, f64)> = match side {
            OrderSide::Buy => &book.asks,
            OrderSide::Sell => &book.bids,
        }
        .iter()
        .filter_map(|lvl| Some((lvl.price.to_f64()?, lvl.size.to_f64()?)))
        .filter(|(_, qty)| *qty > 0.0)
        .collect();

        match side {
            // Buy: cheapest asks first (ascending price).
            OrderSide::Buy => levels.sort_by(|a, b| a.0.total_cmp(&b.0)),
            // Sell: highest bids first (descending price).
            OrderSide::Sell => levels.sort_by(|a, b| b.0.total_cmp(&a.0)),
        }

        if levels.is_empty() {
            anyhow::bail!(
                "Order book for token {} has no liquidity on the {} side",
                token_id,
                match side {
                    OrderSide::Buy => "ask",
                    OrderSide::Sell => "bid",
                }
            );
        }

        walk_levels(&levels, size).with_context(|| {
            format!(
                "Insufficient liquidity for {} shares on token {}",
                size, token_id
            )
        })
    }
}

// =========================================================================================================
// Fill computation
// =========================================================================================================

/// Walk price-priority-sorted `(price, size)` levels, consuming depth until `size` shares
/// are filled, and return the volume-weighted fill. Pure (no IO) so it is unit-testable.
///
/// Errors if the combined depth is short of `size` — never returns a partial fill silently.
fn walk_levels(levels: &[(f64, f64)], size: f64) -> Result<FillResult> {
    let mut remaining = size;
    let mut notional = 0.0;
    for &(price, available) in levels {
        if remaining <= 0.0 {
            break;
        }
        let take = remaining.min(available);
        notional += take * price;
        remaining -= take;
    }

    if remaining > f64::EPSILON {
        anyhow::bail!("insufficient depth (short by {:.4})", remaining);
    }

    Ok(FillResult {
        filled_shares: size,
        avg_price: notional / size,
        total_cost: notional,
    })
}

// =========================================================================================================
// Tests
// =========================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_level_fill() {
        let levels = [(0.50, 100.0)];
        let fill = walk_levels(&levels, 40.0).unwrap();
        assert!((fill.avg_price - 0.50).abs() < 1e-9);
        assert!((fill.total_cost - 20.0).abs() < 1e-9);
        assert!((fill.filled_shares - 40.0).abs() < 1e-9);
    }

    #[test]
    fn multi_level_weighted_average() {
        // 30 @ 0.50, then 20 @ 0.60 for a 50-share order: VWAP = (15 + 12) / 50 = 0.54.
        let levels = [(0.50, 30.0), (0.60, 100.0)];
        let fill = walk_levels(&levels, 50.0).unwrap();
        assert!((fill.avg_price - 0.54).abs() < 1e-9);
        assert!((fill.total_cost - 27.0).abs() < 1e-9);
    }

    #[test]
    fn insufficient_depth_errors() {
        let levels = [(0.50, 10.0), (0.60, 5.0)];
        assert!(walk_levels(&levels, 100.0).is_err());
    }
}
