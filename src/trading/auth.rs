//! CLOB authentication.
//!
//! Provides dynamic authentication with the Polymarket CLOB API using the private key.
//! The SDK handles all credential management internally.
//!
//! Note: this runs at startup, before the TUI owns the terminal, so `println!` is allowed
//! here (see AGENTS.md §4 logging rule).

// =========================================================================================================
// Imports
// =========================================================================================================

use std::str::FromStr;

use anyhow::{Context, Result};
use colored::*;
use polymarket_client_sdk::auth::{LocalSigner, Signer};
use polymarket_client_sdk::{
    clob::{Client, Config},
    POLYGON,
};

// =========================================================================================================
// Types
// =========================================================================================================

/// Wallet address after successful authentication.
pub struct AuthenticatedClient {
    pub wallet_address: String,
}

// =========================================================================================================
// Authentication
// =========================================================================================================

/// Authenticate with the CLOB API using the private key.
///
/// Creates a signer from the private key, verifies the connection by authenticating, and
/// returns the resolved wallet address.
pub async fn authenticate(private_key: &str) -> Result<AuthenticatedClient> {
    println!("{}", "🔐 Authenticating with Polymarket CLOB...".cyan());

    // Create signer from private key
    let signer = LocalSigner::from_str(private_key)
        .context("Failed to parse private key")?
        .with_chain_id(Some(POLYGON));

    let wallet_address = format!("{:#x}", signer.address());
    println!("   Wallet: {}", wallet_address.yellow());

    // Build CLOB client with server time sync
    let config = Config::builder().use_server_time(true).build();

    // Authenticate to verify the connection works
    let _client = Client::new("https://clob.polymarket.com", config)
        .context("Failed to create CLOB client")?
        .authentication_builder(&signer)
        .authenticate()
        .await
        .context("Failed to authenticate. Check your private key and internet connection.")?;

    println!("{}", "✓ CLOB authentication successful!".green().bold());

    Ok(AuthenticatedClient { wallet_address })
}
