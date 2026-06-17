//! Presentation domain: ratatui terminal UI, its screens, and the main event loop.

// =========================================================================================================
// Submodules
// =========================================================================================================

mod app;
pub mod chat;
pub mod config_wizard;
mod events;
mod password_prompt;
pub mod settings;
pub mod theme;
mod ui;

// =========================================================================================================
// Imports
// =========================================================================================================

use std::io::stdout;
use std::sync::Arc;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use crate::trading::ExecutionEngine;

// =========================================================================================================
// Re-exports
// =========================================================================================================

pub use app::App;
pub use chat::ChatState;
pub use config_wizard::ConfigWizard;
pub use events::EventHandler;
pub use password_prompt::PasswordPrompt;
pub use settings::{SettingsAction, SettingsEditor};

// Re-export PendingConfirmation from ai::chatbot so screens can reference it via the tui domain.
pub use crate::ai::chatbot::PendingConfirmation;

// =========================================================================================================
// Lifecycle
// =========================================================================================================

/// Initialize and run the TUI application.
pub async fn run_tui(
    db_pool: crate::data::DbPool,
    execution_engine: Arc<ExecutionEngine>,
    config: &crate::config::SecureConfig,
) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(db_pool, execution_engine, config);
    app.init_watched_markets().await;
    let mut event_handler = EventHandler::new(100); // 100ms tick rate

    // Main loop
    let result = run_app(&mut terminal, &mut app, &mut event_handler).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    event_handler: &mut EventHandler,
) -> Result<()> {
    loop {
        // Draw UI
        terminal.draw(|frame| ui::draw(frame, app))?;

        // Handle events
        if let Some(event) = event_handler.next()? {
            app.handle_event(event).await?;
        }

        // Check if we should quit
        if app.should_quit {
            break;
        }

        // Refresh data periodically
        app.refresh_data().await;
    }

    Ok(())
}
