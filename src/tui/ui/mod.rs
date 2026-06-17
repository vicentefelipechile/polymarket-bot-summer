//! TUI rendering domain. Render-only: reads `App` state and draws via `theme` helpers,
//! never mutates state or performs IO (that lives in `app.rs`). All styling goes through
//! `theme`.
//!
//! This `mod.rs` does wiring + the top-level frame layout (`draw`) and the per-tab
//! dispatcher (`draw_content`) only. Each panel's rendering lives in its own submodule.

// =========================================================================================================
// Modules
// =========================================================================================================

mod chrome;
mod dashboard;
mod docs;
mod logs;
mod markets;
mod modals;
mod orders;
mod positions;

// =========================================================================================================
// Imports
// =========================================================================================================

use ratatui::{prelude::*, widgets::Paragraph};

use crate::tui::app::{App, InputMode, Tab};

// =========================================================================================================
// Top-level layout
// =========================================================================================================

/// Draw the complete TUI.
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Main layout: Header, Tabs, Content, Command Input (if active), Footer
    let constraints = if app.input_mode == InputMode::Command {
        vec![
            Constraint::Length(3), // Header
            Constraint::Length(3), // Tabs
            Constraint::Min(8),    // Content
            Constraint::Length(3), // Command input
            Constraint::Length(3), // Footer
        ]
    } else {
        vec![
            Constraint::Length(3), // Header
            Constraint::Length(3), // Tabs
            Constraint::Min(10),   // Content
            Constraint::Length(3), // Footer
        ]
    };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    chrome::draw_header(frame, layout[0], app);
    chrome::draw_tabs(frame, layout[1], app);
    draw_content(frame, layout[2], app);

    if app.input_mode == InputMode::Command {
        chrome::draw_command_input(frame, layout[3], app);
        chrome::draw_footer(frame, layout[4], app);
    } else {
        chrome::draw_footer(frame, layout[3], app);
    }

    // Draw quit confirmation modal on top if active
    if app.input_mode == InputMode::QuitConfirmation {
        modals::draw_quit_confirmation_modal(frame, area, app);
    }

    // Draw leave market confirmation modal on top if active
    if app.input_mode == InputMode::LeaveMarketConfirmation {
        modals::draw_leave_confirmation_modal(frame, area, app);
    }

    // Draw trade-entry form on top if active
    if app.input_mode == InputMode::TradeEntry {
        modals::draw_trade_entry_modal(frame, area, app);
    }
}

/// Dispatch the content area to the active tab's renderer.
fn draw_content(frame: &mut Frame, area: Rect, app: &App) {
    match app.current_tab {
        Tab::Dashboard => dashboard::draw_dashboard(frame, area, app),
        Tab::Orders => orders::draw_orders(frame, area, app),
        Tab::Positions => positions::draw_positions(frame, area, app),
        Tab::Markets => markets::draw_markets(frame, area, app),
        Tab::MarketDetail => markets::draw_market_detail(frame, area, app),
        Tab::Logs => logs::draw_logs(frame, area, app),
        Tab::Docs => docs::draw_docs(frame, area, app),
        Tab::AiChat => draw_ai_chat(frame, area, app),
        Tab::Settings => app.settings_editor.render(frame, area),
    }
}

/// Render the AI chat panel, or a disabled placeholder when no API key is configured.
///
/// Kept here (not in a submodule) because it only delegates to `crate::tui::chat`.
fn draw_ai_chat(frame: &mut Frame, area: Rect, app: &App) {
    use crate::tui::theme::{self, palette};

    if app.chat_request_tx.is_some() {
        // Use the chat rendering module
        crate::tui::chat::render_chat(frame, area, &app.chat_state);
    } else {
        // AI not enabled
        let msg = Paragraph::new(vec![
            Line::raw(""),
            Line::styled(
                "  AI Chat is not enabled (API Key missing)",
                Style::default().fg(Color::Yellow),
            ),
            Line::raw(""),
        ])
        .block(theme::titled_block("AI Chat Disabled", palette::MUTED))
        .alignment(Alignment::Center);
        frame.render_widget(msg, area);
    }
}
