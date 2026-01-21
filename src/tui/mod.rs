mod app;
mod events;
mod ui;

pub use app::App;
pub use events::EventHandler;

use crate::execution::ExecutionEngine;
use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io::stdout;
use std::sync::Arc;

/// Initialize and run the TUI application
pub async fn run_tui(execution_engine: Arc<ExecutionEngine>) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(execution_engine);
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
