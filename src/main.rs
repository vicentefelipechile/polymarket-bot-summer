use anyhow::Result;
use polymarket_bot_summer::{
    authenticate, init_database, run_onboarding_checks, run_tui, Config, ExecutionEngine,
    SpikeDetector,
};
use std::fs::File;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to file (not terminal, to avoid corrupting TUI)
    let log_file = File::create("bot.log").unwrap_or_else(|_| {
        // Fallback: if we can't create file, just disable file logging
        File::open(if cfg!(windows) { "NUL" } else { "/dev/null" }).unwrap()
    });

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::sync::Mutex::new(log_file))
                .with_ansi(false),
        )
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Run onboarding checks (validates private key and database)
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

    // Authenticate with CLOB API (dynamic, no stored credentials needed)
    let auth_client = match authenticate(&config.private_key).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("CLOB authentication failed: {}", e);
            std::process::exit(1);
        }
    };
    tracing::info!("✓ Authenticated as {}", auth_client.wallet_address);

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

    // TODO: Integrate auth_client with polymarket-hft for actual trading
    tracing::info!("⚠ Trading integration pending - running in demo mode");

    // Start TUI
    run_tui(db, execution_engine).await?;

    Ok(())
}
