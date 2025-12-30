use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};

pub type DbPool = Pool<Sqlite>;

/// Initialize the database with WAL mode for high concurrency
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

/// Create database schema
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
    
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_orderbook_market ON orderbook_snapshots(market_id)")
        .execute(pool)
        .await?;
    
    Ok(())
}
