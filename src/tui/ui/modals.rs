//! Confirmation modals drawn on top of the active panel (quit, leave market).

// =========================================================================================================
// Imports
// =========================================================================================================

use ratatui::prelude::*;

use crate::tui::app::{App, LeaveSelection, QuitSelection};
use crate::tui::theme;

// =========================================================================================================
// Rendering
// =========================================================================================================

pub(super) fn draw_quit_confirmation_modal(frame: &mut Frame, area: Rect, app: &App) {
    let body = vec![Line::from(Span::styled(
        "Are you sure you want to quit?",
        theme::fg_bold(theme::palette::SELECTED),
    ))];

    theme::confirm_modal(
        frame,
        area,
        "⚠️  Confirm Quit",
        theme::palette::DANGER,
        body,
        app.quit_selection == QuitSelection::Yes,
    );
}

pub(super) fn draw_leave_confirmation_modal(frame: &mut Frame, area: Rect, app: &App) {
    let market_name = app
        .watched_markets_info
        .get(app.selected_watched_market_index)
        .map(|m| theme::truncate(&m.question, 40))
        .unwrap_or_else(|| "Unknown".to_string());

    let body = vec![
        Line::from(Span::styled(
            "Leave this market?",
            theme::fg_bold(theme::palette::SELECTED),
        )),
        Line::raw(""),
        Line::from(Span::styled(
            market_name,
            theme::fg(theme::palette::PRIMARY),
        )),
    ];

    theme::confirm_modal(
        frame,
        area,
        "🚪 Leave Market",
        theme::palette::SELECTED,
        body,
        app.leave_selection == LeaveSelection::Yes,
    );
}
