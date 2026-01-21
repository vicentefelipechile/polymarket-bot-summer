use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    // Authentication - only private key needed, CLOB auth is dynamic
    pub private_key: String,

    // Trading parameters
    pub max_order_size: f64,
    pub min_order_size: f64,
    pub volume_velocity_threshold: f64,
    pub obi_threshold: f64,

    // System
    pub database_path: String,
    pub rpc_url: Option<String>,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            // Only private key is required - CLOB credentials are generated dynamically
            private_key: env::var("POLYMARKET_PK").context("POLYMARKET_PK not found")?,

            // Trading parameters with defaults
            max_order_size: env::var("MAX_ORDER_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100.0),
            min_order_size: env::var("MIN_ORDER_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1.0),
            volume_velocity_threshold: env::var("VOLUME_VELOCITY_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000.0),
            obi_threshold: env::var("OBI_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.3),

            // System configuration
            database_path: env::var("DATABASE_PATH")
                .unwrap_or_else(|_| "./bot_history.db".to_string()),
            rpc_url: env::var("RPC_URL").ok(),
        })
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate private key format
        if !self.private_key.starts_with("0x") {
            anyhow::bail!("Private key must start with '0x'");
        }

        // Validate order sizes
        if self.min_order_size <= 0.0 {
            anyhow::bail!("MIN_ORDER_SIZE must be greater than 0");
        }

        if self.max_order_size < self.min_order_size {
            anyhow::bail!("MAX_ORDER_SIZE must be greater than MIN_ORDER_SIZE");
        }

        // Validate OBI threshold
        if self.obi_threshold < -1.0 || self.obi_threshold > 1.0 {
            anyhow::bail!("OBI_THRESHOLD must be between -1.0 and 1.0");
        }

        Ok(())
    }
}
