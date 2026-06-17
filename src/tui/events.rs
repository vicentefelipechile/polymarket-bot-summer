//! Keyboard event polling for the TUI event loop.

// =========================================================================================================
// Imports
// =========================================================================================================

use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent, KeyEventKind};

// =========================================================================================================
// Types
// =========================================================================================================

/// Event handler for keyboard input.
pub struct EventHandler {
    tick_rate: Duration,
}

// =========================================================================================================
// Implementation
// =========================================================================================================

impl EventHandler {
    pub fn new(tick_rate_ms: u64) -> Self {
        Self {
            tick_rate: Duration::from_millis(tick_rate_ms),
        }
    }

    /// Poll for the next keyboard event (only Press events, ignoring Release/Repeat).
    // Not an `Iterator`: this is a one-shot poll with a tick timeout, named `next` for clarity.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> std::io::Result<Option<KeyEvent>> {
        if event::poll(self.tick_rate)? {
            if let Event::Key(key_event) = event::read()? {
                // Only handle key Press events, ignore Release and Repeat.
                if key_event.kind == KeyEventKind::Press {
                    return Ok(Some(key_event));
                }
            }
        }
        Ok(None)
    }
}
