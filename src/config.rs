use crate::crypto::{AiPersonality, SecureConfig};
use anyhow::{Context, Result};
use std::env;
use std::path::Path;

impl SecureConfig {
    /// Migrate configuration from environment variables (legacy .env file)
    pub fn migrate_from_env() -> Result<Self> {
        Ok(SecureConfig {
            // Authentication - only private key needed, CLOB auth is dynamic
            private_key: env::var("POLYMARKET_PK").context("POLYMARKET_PK not found in .env")?,

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

            // AI configuration (new, defaults for migration)
            gemini_api_key: env::var("GEMINI_API_KEY").ok(),
            ai_personality: AiPersonality::default(),
            ai_enabled: false, // Disabled by default on migration
            ai_analysis_frequency_minutes: 30,
        })
    }

    /// Load configuration from encrypted file
    pub fn from_encrypted_file(path: &Path, password: &str) -> Result<Self> {
        crate::crypto::load_config(path, password)
    }

    /// Save configuration to encrypted file
    pub fn save_to_file(&self, path: &Path, password: &str) -> Result<()> {
        crate::crypto::save_config(self, path, password)
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

        // Validate AI settings
        if self.ai_enabled && self.gemini_api_key.is_none() {
            anyhow::bail!("AI is enabled but GEMINI_API_KEY is not set");
        }

        if self.ai_analysis_frequency_minutes == 0 {
            anyhow::bail!("AI analysis frequency must be greater than 0");
        }

        Ok(())
    }

    /// Create a default configuration for testing
    #[cfg(test)]
    pub fn default_test_config() -> Self {
        Self {
            private_key: "0x0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            max_order_size: 100.0,
            min_order_size: 1.0,
            volume_velocity_threshold: 1000.0,
            obi_threshold: 0.3,
            database_path: "./test_bot_history.db".to_string(),
            rpc_url: None,
            gemini_api_key: None,
            ai_personality: AiPersonality::Summer,
            ai_enabled: false,
            ai_analysis_frequency_minutes: 30,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_config() {
        let config = SecureConfig::default_test_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_private_key() {
        let mut config = SecureConfig::default_test_config();
        config.private_key = "invalid_key".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_order_sizes() {
        let mut config = SecureConfig::default_test_config();
        config.min_order_size = 0.0;
        assert!(config.validate().is_err());

        config.min_order_size = 100.0;
        config.max_order_size = 50.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_ai_enabled_without_key() {
        let mut config = SecureConfig::default_test_config();
        config.ai_enabled = true;
        config.gemini_api_key = None;
        assert!(config.validate().is_err());
    }
}
