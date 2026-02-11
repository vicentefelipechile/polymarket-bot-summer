//! Main entry point for Polymarket Bot Summer

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use polymarket_bot_summer::{
    authenticate, init_database, run_tui, ConfigWizard, ExecutionEngine, GeminiClient,
    MarketAnalyzer, PasswordPrompt, SecureConfig, SpikeDetector,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::fs::File;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const CONFIG_PATH: &str = "./summer.bot";

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to file (not terminal, to avoid corrupting TUI)
    let log_file = File::create("bot.log")
        .unwrap_or_else(|_| File::open(if cfg!(windows) { "NUL" } else { "/dev/null" }).unwrap());

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

    // Load or create configuration
    let config = load_or_create_config().await?;

    // Validate configuration
    config
        .validate()
        .context("Configuration validation failed")?;

    tracing::info!("✓ Configuration loaded and validated");

    // Authenticate with CLOB API
    let auth_client = authenticate(&config.private_key)
        .await
        .context("CLOB authentication failed")?;
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

    // Initialize AI if enabled
    if config.ai_enabled {
        if let Some(ref api_key) = config.gemini_api_key {
            let client = GeminiClient::new(api_key.clone());
            let analyzer = Arc::new(MarketAnalyzer::new(
                client,
                config.ai_personality,
                db.clone(),
            ));
            tracing::info!(
                "✓ AI analyzer initialized with {} personality",
                config.ai_personality
            );

            // Start periodic AI analysis task
            let analyzer_clone = analyzer.clone();
            let frequency = config.ai_analysis_frequency_minutes;
            tokio::spawn(async move {
                periodic_ai_analysis(analyzer_clone, frequency).await;
            });
        } else {
            tracing::warn!("⚠️  AI enabled but no Gemini API key configured");
        }
    } else {
        tracing::info!("AI analysis disabled");
    }

    // TODO: Integrate auth_client with polymarket-hft for actual trading
    tracing::info!("⚠ Trading integration pending - running in demo mode");

    // Start TUI
    run_tui(db, execution_engine, &config).await?;

    Ok(())
}

/// Load existing config or run first-time setup wizard
async fn load_or_create_config() -> Result<SecureConfig> {
    let config_path = Path::new(CONFIG_PATH);

    if config_path.exists() {
        // Config exists, prompt for password
        prompt_for_password(config_path).await
    } else {
        // First time setup - run wizard
        run_first_time_setup(config_path).await
    }
}

/// Load configuration with password prompt
async fn prompt_for_password(config_path: &Path) -> Result<SecureConfig> {
    // Setup terminal for password prompt
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut prompt = PasswordPrompt::new();
    let _password: Option<String> = None; // For future use (e.g., re-auth)
    let max_attempts = 3;
    let mut attempts = 0;

    // Password input loop
    while _password.is_none() && attempts < max_attempts {
        terminal.draw(|f| {
            prompt.render(f, f.area());
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Check for Ctrl+C or ESC to exit
                if (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
                    || key.code == KeyCode::Esc
                {
                    cleanup_terminal(&mut terminal)?;
                    std::process::exit(0);
                }

                if let Some(pwd) = prompt.handle_input(key) {
                    // Try to load config with this password
                    match SecureConfig::from_encrypted_file(config_path, &pwd) {
                        Ok(config) => {
                            cleanup_terminal(&mut terminal)?;
                            return Ok(config);
                        }
                        Err(_) => {
                            attempts += 1;
                            if attempts < max_attempts {
                                prompt.set_error(format!(
                                    "Contraseña incorrecta. Intentos restantes: {}",
                                    max_attempts - attempts
                                ));
                            } else {
                                cleanup_terminal(&mut terminal)?;
                                anyhow::bail!("Máximo de intentos de contraseña alcanzado");
                            }
                        }
                    }
                }
            }
        }
    }

    cleanup_terminal(&mut terminal)?;
    anyhow::bail!("Failed to load configuration")
}

/// Run first-time setup wizard
async fn run_first_time_setup(config_path: &Path) -> Result<SecureConfig> {
    // Setup terminal for wizard
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut wizard = ConfigWizard::new();
    let mut config: Option<SecureConfig> = None;

    // Wizard loop
    while config.is_none() {
        terminal.draw(|f| {
            wizard.render(f, f.area());
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if let Some(completed_config) = wizard.handle_input(key) {
                    config = Some(completed_config);
                }
            }
        }
    }

    cleanup_terminal(&mut terminal)?;

    // Save encrypted configuration with password from wizard
    let config = config.unwrap();
    let password = wizard.get_password();
    config
        .save_to_file(config_path, password)
        .context("Failed to save encrypted configuration")?;

    tracing::info!("✓ Configuración guardada exitosamente");

    Ok(config)
}

/// Cleanup terminal state
fn cleanup_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Periodic AI analysis of watched markets
async fn periodic_ai_analysis(_analyzer: Arc<MarketAnalyzer>, frequency_minutes: u32) {
    let mut interval = tokio::time::interval(Duration::from_secs(frequency_minutes as u64 * 60));

    loop {
        interval.tick().await;

        tracing::info!("🤖 Running periodic AI analysis...");

        // TODO: Get watched markets from database and analyze them using _analyzer
        // Example: Use _analyzer to analyze specific markets
        // let markets = load_watched_markets(&pool).await?;
        // for market in markets {
        //     let _ = _analyzer.analyze_market(&market).await;
        // }

        tracing::info!("⏰ AI analysis cycle completed");
    }
}
