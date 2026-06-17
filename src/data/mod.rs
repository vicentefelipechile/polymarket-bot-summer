//! Data domain: SQLite persistence and shared domain types.

// =========================================================================================================
// Submodules
// =========================================================================================================

pub mod database;
pub mod types;

// =========================================================================================================
// Re-exports
// =========================================================================================================

pub use database::{
    ensure_simulation_account, get_position, get_simulation_account, init_database, load_orders,
    load_positions, update_simulation_account, upsert_order, upsert_position, DbPool,
};
pub use types::{
    BotState, ExecutionMode, OrderBookImbalance, OrderInfo, OrderSide, OrderType, Portfolio,
    Position, VolumeVelocityEvent,
};
