//! Trading domain: CLOB authentication, order execution, market data, and spike detection.

// =========================================================================================================
// Submodules
// =========================================================================================================

pub mod auth;
pub mod execution;
pub mod markets;
pub mod order_book;
pub mod simulation;
pub mod spike_detection;

// =========================================================================================================
// Re-exports
// =========================================================================================================

pub use auth::{authenticate, AuthenticatedClient};
pub use execution::{ExecutionEngine, TradeRequest};
pub use markets::{MarketInfo, MarketService};
pub use order_book::{FillResult, OrderBookService};
pub use simulation::SimulationEngine;
pub use spike_detection::SpikeDetector;
