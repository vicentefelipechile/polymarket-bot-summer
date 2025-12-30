use anyhow::Result;
use polymarket_hft_bot::{
    run_onboarding_checks, Config, ExecutionEngine, CLI,
    init_database, SpikeDetector,
};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    // Load environment variables
    dotenvy::dotenv().ok();
    
    // Run onboarding checks
    if let Err(e) = run_onboarding_checks() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
    
    // Load and validate configuration
    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };
    
    if let Err(e) = config.validate() {
        eprintln!("Configuration validation failed: {}", e);
        std::process::exit(1);
    }
    
    // Initialize database
    let db = init_database(&config.database_path).await?;
    tracing::info!("✓ Database initialized at {}", config.database_path);
    
    // Initialize spike detector
    let _spike_detector = SpikeDetector::new(
        db.clone(),
        config.volume_velocity_threshold,
        config.obi_threshold,
    );
    tracing::info!("✓ Spike detector initialized");
    
    // Initialize execution engine
    let execution_engine = Arc::new(ExecutionEngine::new(config.clone()));
    tracing::info!("✓ Execution engine initialized");
    
    // TODO: Initialize polymarket-hft client
    // This will be added in a future update when integrating with the actual API
    tracing::info!("⚠ polymarket-hft integration pending - running in demo mode");
    
    // Start CLI REPL
    let mut cli = CLI::new(execution_engine)?;
    cli.run().await?;
    
    Ok(())
}
