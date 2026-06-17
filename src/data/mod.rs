//! Data domain: SQLite persistence and shared domain types.

// =========================================================================================================
// Submodules
// =========================================================================================================

pub mod database;
pub mod types;

// =========================================================================================================
// Re-exports
// =========================================================================================================

pub use database::{init_database, DbPool};
pub use types::{BotState, OrderBookImbalance, OrderInfo, Portfolio, VolumeVelocityEvent};
