//! SQLite persistence: connection pool, WAL mode, and schema creation.

// =========================================================================================================
// Imports
// =========================================================================================================

use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Pool, Row, Sqlite};

use crate::data::types::{ExecutionMode, OrderInfo, Position};

// =========================================================================================================
// Types
// =========================================================================================================

pub type DbPool = Pool<Sqlite>;

// =========================================================================================================
// Initialization
// =========================================================================================================

/// Initialize the database with WAL mode for high concurrency.
pub async fn init_database(database_path: &str) -> Result<DbPool> {
    // Configure SQLite with WAL mode
    let options = SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);

    // Create connection pool
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    // Run migrations to create tables
    create_schema(&pool).await?;

    Ok(pool)
}

// =========================================================================================================
// Schema
// =========================================================================================================

/// Create database schema (idempotent: `CREATE TABLE IF NOT EXISTS`).
async fn create_schema(pool: &DbPool) -> Result<()> {
    // Markets table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS markets (
            market_id TEXT PRIMARY KEY,
            question TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            end_date INTEGER,
            active BOOLEAN NOT NULL DEFAULT 1
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Trades table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS trades (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            market_id TEXT NOT NULL,
            trade_id TEXT NOT NULL,
            side TEXT NOT NULL,
            price REAL NOT NULL,
            size REAL NOT NULL,
            timestamp INTEGER NOT NULL,
            FOREIGN KEY (market_id) REFERENCES markets(market_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Orders table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS orders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            order_id TEXT NOT NULL UNIQUE,
            market_id TEXT NOT NULL,
            side TEXT NOT NULL,
            order_type TEXT NOT NULL,
            price REAL NOT NULL,
            size REAL NOT NULL,
            filled_size REAL NOT NULL DEFAULT 0,
            status TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY (market_id) REFERENCES markets(market_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Order book snapshots table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS orderbook_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            market_id TEXT NOT NULL,
            bids_volume REAL NOT NULL,
            asks_volume REAL NOT NULL,
            best_bid REAL,
            best_ask REAL,
            timestamp INTEGER NOT NULL,
            FOREIGN KEY (market_id) REFERENCES markets(market_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Volume velocity events table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS volume_velocity_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            market_id TEXT NOT NULL,
            velocity REAL NOT NULL,
            volume_delta REAL NOT NULL,
            time_delta REAL NOT NULL,
            timestamp INTEGER NOT NULL,
            FOREIGN KEY (market_id) REFERENCES markets(market_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Portfolio snapshots table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS portfolio_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            usdc_balance REAL NOT NULL,
            total_value REAL NOT NULL,
            realized_pnl REAL NOT NULL DEFAULT 0,
            unrealized_pnl REAL NOT NULL DEFAULT 0,
            timestamp INTEGER NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Watched markets table - persists joined markets across sessions
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS watched_markets (
            id TEXT PRIMARY KEY,
            question TEXT NOT NULL,
            volume TEXT,
            outcomes TEXT,
            prices TEXT,
            token_ids TEXT,
            joined_at INTEGER NOT NULL,
            active BOOLEAN NOT NULL DEFAULT 1
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Positions table - open holdings of outcome shares, real or simulated.
    // A position is uniquely identified by (market_id, outcome_index, execution_mode):
    // the same outcome can be held both for real and in simulation simultaneously.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS positions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            market_id TEXT NOT NULL,
            outcome_index INTEGER NOT NULL,
            outcome_label TEXT NOT NULL,
            shares REAL NOT NULL,
            avg_price REAL NOT NULL,
            realized_pnl REAL NOT NULL DEFAULT 0,
            execution_mode TEXT NOT NULL DEFAULT 'REAL',
            updated_at INTEGER NOT NULL,
            UNIQUE (market_id, outcome_index, execution_mode)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Simulation account - the separate virtual wallet for paper trading. A single row
    // holds the running balance; `starting_balance` records what it was seeded with.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS simulation_account (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            balance_usdc REAL NOT NULL,
            starting_balance REAL NOT NULL,
            realized_pnl REAL NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // AI recommendations table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS ai_recommendations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            market_id TEXT NOT NULL,
            action TEXT NOT NULL,
            confidence REAL NOT NULL,
            reasoning TEXT NOT NULL,
            analysis TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            personality TEXT NOT NULL,
            FOREIGN KEY (market_id) REFERENCES markets(market_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // AI chat history table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS ai_chat_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp INTEGER NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indices for better query performance
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_trades_market ON trades(market_id)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_trades_timestamp ON trades(timestamp)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_orders_market ON orders(market_id)")
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_orderbook_market ON orderbook_snapshots(market_id)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_positions_market ON positions(market_id, execution_mode)",
    )
    .execute(pool)
    .await?;

    // Bring older databases up to the current schema. `CREATE TABLE IF NOT EXISTS` never
    // adds columns to a table that already exists, so new columns are added explicitly.
    run_column_migrations(pool).await?;

    Ok(())
}

/// Add columns introduced after a table's original `CREATE` for already-existing DBs.
///
/// SQLite has no `ADD COLUMN IF NOT EXISTS`; re-adding an existing column errors with
/// "duplicate column name". That specific error is expected on an up-to-date DB and is
/// swallowed — any other error propagates.
async fn run_column_migrations(pool: &DbPool) -> Result<()> {
    let migrations = [
        "ALTER TABLE orders ADD COLUMN execution_mode TEXT NOT NULL DEFAULT 'REAL'",
        "ALTER TABLE orders ADD COLUMN outcome_index INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE watched_markets ADD COLUMN token_ids TEXT",
    ];

    for stmt in migrations {
        if let Err(e) = sqlx::query(stmt).execute(pool).await {
            let msg = e.to_string().to_lowercase();
            if msg.contains("duplicate column") {
                // Column already present — DB is already migrated for this statement.
                continue;
            }
            return Err(
                anyhow::Error::from(e).context(format!("Schema migration failed: {}", stmt))
            );
        }
    }

    Ok(())
}

// =========================================================================================================
// Orders persistence
// =========================================================================================================

/// Ensure a `markets` row exists for `market_id` so order/position inserts satisfy the
/// `FOREIGN KEY (market_id) REFERENCES markets(market_id)` constraint (enforced by SQLite
/// when foreign keys are on). A no-op when the row already exists.
async fn ensure_market(pool: &DbPool, market_id: &str) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO markets (market_id, question, created_at, active)
        VALUES (?, '', ?, 1)
        ON CONFLICT(market_id) DO NOTHING
        "#,
    )
    .bind(market_id)
    .bind(chrono::Utc::now().timestamp())
    .execute(pool)
    .await?;

    Ok(())
}

/// Insert or update an order row (keyed by `order_id`).
pub async fn upsert_order(pool: &DbPool, order: &OrderInfo) -> Result<()> {
    ensure_market(pool, &order.market_id).await?;
    sqlx::query(
        r#"
        INSERT INTO orders
        (order_id, market_id, side, order_type, price, size, filled_size, status,
         execution_mode, outcome_index, created_at, updated_at)
        VALUES (?, ?, ?, 'MARKET', ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(order_id) DO UPDATE SET
            filled_size = excluded.filled_size,
            status = excluded.status,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(&order.order_id)
    .bind(&order.market_id)
    .bind(&order.side)
    .bind(order.price)
    .bind(order.size)
    .bind(order.filled_size)
    .bind(&order.status)
    .bind(order.execution_mode.as_str())
    .bind(order.outcome_index as i64)
    .bind(order.created_at)
    .bind(chrono::Utc::now().timestamp())
    .execute(pool)
    .await?;

    Ok(())
}

/// Load all orders (real + simulated), newest first.
pub async fn load_orders(pool: &DbPool) -> Result<Vec<OrderInfo>> {
    let rows = sqlx::query(
        r#"
        SELECT order_id, market_id, side, price, size, filled_size, status,
               execution_mode, outcome_index, created_at
        FROM orders
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut orders = Vec::with_capacity(rows.len());
    for row in rows {
        let mode_str: String = row.get("execution_mode");
        let outcome_index: i64 = row.get("outcome_index");
        orders.push(OrderInfo {
            order_id: row.get("order_id"),
            market_id: row.get("market_id"),
            side: row.get("side"),
            price: row.get("price"),
            size: row.get("size"),
            filled_size: row.get("filled_size"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            execution_mode: ExecutionMode::parse_str(&mode_str),
            outcome_index: outcome_index.max(0) as usize,
        });
    }

    Ok(orders)
}

// =========================================================================================================
// Positions persistence
// =========================================================================================================

/// Insert or update a position, keyed by `(market_id, outcome_index, execution_mode)`.
pub async fn upsert_position(pool: &DbPool, position: &Position) -> Result<()> {
    ensure_market(pool, &position.market_id).await?;
    sqlx::query(
        r#"
        INSERT INTO positions
        (market_id, outcome_index, outcome_label, shares, avg_price, realized_pnl,
         execution_mode, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(market_id, outcome_index, execution_mode) DO UPDATE SET
            outcome_label = excluded.outcome_label,
            shares = excluded.shares,
            avg_price = excluded.avg_price,
            realized_pnl = excluded.realized_pnl,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(&position.market_id)
    .bind(position.outcome_index as i64)
    .bind(&position.outcome_label)
    .bind(position.shares)
    .bind(position.avg_price)
    .bind(position.realized_pnl)
    .bind(position.mode.as_str())
    .bind(chrono::Utc::now().timestamp())
    .execute(pool)
    .await?;

    Ok(())
}

/// Fetch a single position by its identity, if it exists.
pub async fn get_position(
    pool: &DbPool,
    market_id: &str,
    outcome_index: usize,
    mode: ExecutionMode,
) -> Result<Option<Position>> {
    let row = sqlx::query(
        r#"
        SELECT market_id, outcome_index, outcome_label, shares, avg_price, realized_pnl,
               execution_mode
        FROM positions
        WHERE market_id = ? AND outcome_index = ? AND execution_mode = ?
        "#,
    )
    .bind(market_id)
    .bind(outcome_index as i64)
    .bind(mode.as_str())
    .fetch_optional(pool)
    .await?;

    Ok(row.map(row_to_position))
}

/// Load all positions for a given execution mode, including fully-closed ones (shares = 0)
/// so realized PnL stays visible.
pub async fn load_positions(pool: &DbPool, mode: ExecutionMode) -> Result<Vec<Position>> {
    let rows = sqlx::query(
        r#"
        SELECT market_id, outcome_index, outcome_label, shares, avg_price, realized_pnl,
               execution_mode
        FROM positions
        WHERE execution_mode = ?
        ORDER BY updated_at DESC
        "#,
    )
    .bind(mode.as_str())
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(row_to_position).collect())
}

/// Map a positions row to a `Position`.
fn row_to_position(row: sqlx::sqlite::SqliteRow) -> Position {
    let mode_str: String = row.get("execution_mode");
    let outcome_index: i64 = row.get("outcome_index");
    Position {
        market_id: row.get("market_id"),
        outcome_index: outcome_index.max(0) as usize,
        outcome_label: row.get("outcome_label"),
        shares: row.get("shares"),
        avg_price: row.get("avg_price"),
        realized_pnl: row.get("realized_pnl"),
        mode: ExecutionMode::parse_str(&mode_str),
    }
}

// =========================================================================================================
// Simulation account persistence
// =========================================================================================================

/// Ensure the simulation wallet row exists, seeding it with `starting_balance` on first use.
/// Returns the current balance (existing balance if already present).
pub async fn ensure_simulation_account(pool: &DbPool, starting_balance: f64) -> Result<f64> {
    sqlx::query(
        r#"
        INSERT INTO simulation_account (id, balance_usdc, starting_balance, realized_pnl, created_at)
        VALUES (1, ?, ?, 0, ?)
        ON CONFLICT(id) DO NOTHING
        "#,
    )
    .bind(starting_balance)
    .bind(starting_balance)
    .bind(chrono::Utc::now().timestamp())
    .execute(pool)
    .await?;

    get_simulation_account(pool)
        .await
        .map(|opt| opt.map(|a| a.0).unwrap_or(starting_balance))
}

/// Fetch the simulation wallet as `(balance_usdc, realized_pnl)`, if seeded.
pub async fn get_simulation_account(pool: &DbPool) -> Result<Option<(f64, f64)>> {
    let row = sqlx::query("SELECT balance_usdc, realized_pnl FROM simulation_account WHERE id = 1")
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| (r.get("balance_usdc"), r.get("realized_pnl"))))
}

/// Overwrite the simulation wallet's balance and realized PnL.
pub async fn update_simulation_account(
    pool: &DbPool,
    balance_usdc: f64,
    realized_pnl: f64,
) -> Result<()> {
    sqlx::query("UPDATE simulation_account SET balance_usdc = ?, realized_pnl = ? WHERE id = 1")
        .bind(balance_usdc)
        .bind(realized_pnl)
        .execute(pool)
        .await?;

    Ok(())
}

// =========================================================================================================
// Tests
// =========================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// An in-memory pool with the full schema applied — isolated per test.
    async fn test_pool() -> DbPool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite");
        create_schema(&pool).await.expect("schema");
        pool
    }

    #[tokio::test]
    async fn order_execution_mode_roundtrips_through_db() {
        let pool = test_pool().await;
        let order = OrderInfo {
            order_id: "sim_1".to_string(),
            market_id: "mkt".to_string(),
            side: "BUY".to_string(),
            price: 0.42,
            size: 10.0,
            filled_size: 10.0,
            status: "FILLED".to_string(),
            created_at: 1,
            execution_mode: ExecutionMode::Simulated,
            outcome_index: 2,
        };
        upsert_order(&pool, &order).await.unwrap();

        let loaded = load_orders(&pool).await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].execution_mode, ExecutionMode::Simulated);
        assert_eq!(loaded[0].outcome_index, 2);
    }

    #[tokio::test]
    async fn position_upsert_is_keyed_by_mode() {
        let pool = test_pool().await;
        let real = Position::empty("mkt".to_string(), 0, "Yes".to_string(), ExecutionMode::Real);
        let mut sim = Position::empty(
            "mkt".to_string(),
            0,
            "Yes".to_string(),
            ExecutionMode::Simulated,
        );
        sim.shares = 5.0;

        upsert_position(&pool, &real).await.unwrap();
        upsert_position(&pool, &sim).await.unwrap();

        // Same (market, outcome) but different modes are distinct rows.
        let real_back = get_position(&pool, "mkt", 0, ExecutionMode::Real)
            .await
            .unwrap()
            .unwrap();
        let sim_back = get_position(&pool, "mkt", 0, ExecutionMode::Simulated)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(real_back.shares, 0.0);
        assert_eq!(sim_back.shares, 5.0);
    }

    #[tokio::test]
    async fn simulation_account_seeds_once_and_updates() {
        let pool = test_pool().await;
        let seeded = ensure_simulation_account(&pool, 1000.0).await.unwrap();
        assert_eq!(seeded, 1000.0);

        // Re-seeding with a different starting balance must not overwrite the existing one.
        let again = ensure_simulation_account(&pool, 9999.0).await.unwrap();
        assert_eq!(again, 1000.0);

        update_simulation_account(&pool, 750.0, -12.5)
            .await
            .unwrap();
        let (balance, pnl) = get_simulation_account(&pool).await.unwrap().unwrap();
        assert_eq!(balance, 750.0);
        assert_eq!(pnl, -12.5);
    }

    #[tokio::test]
    async fn column_migrations_are_idempotent() {
        let pool = test_pool().await;
        // Running again on an already-migrated DB must not error (duplicate-column tolerated).
        run_column_migrations(&pool).await.unwrap();
    }
}
