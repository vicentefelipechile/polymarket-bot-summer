//! Positions panel: open holdings (real and simulated) with cost basis and realized PnL.

// =========================================================================================================
// Imports
// =========================================================================================================

use ratatui::{
    prelude::*,
    widgets::{List, ListItem},
};

use crate::data::ExecutionMode;
use crate::tui::app::App;
use crate::tui::theme::{self, palette};

// =========================================================================================================
// Constants
// =========================================================================================================

/// Max characters of a market id shown before truncation.
const MARKET_ID_WIDTH: usize = 14;

// =========================================================================================================
// Rendering
// =========================================================================================================

pub(super) fn draw_positions(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = if app.positions.is_empty() {
        vec![ListItem::new(Line::styled(
            "  No open positions",
            theme::fg(palette::SELECTED),
        ))]
    } else {
        app.positions.iter().map(position_row).collect()
    };

    let positions_list = List::new(items).block(theme::titled_block(
        &format!("📊 Positions ({})", app.positions.len()),
        palette::PRIMARY,
    ));

    frame.render_widget(positions_list, area);
}

/// Build one list row: `[SIMULATED] <market> | outcome | shares @ avg | PnL`.
fn position_row(position: &crate::data::Position) -> ListItem<'static> {
    let mut spans = vec![Span::raw("  ")];

    if position.mode == ExecutionMode::Simulated {
        spans.push(Span::styled(
            " SIMULATED ",
            theme::pill(palette::ACCENT, palette::TEXT),
        ));
        spans.push(Span::raw(" "));
    }

    spans.push(Span::styled(
        theme::truncate(&position.market_id, MARKET_ID_WIDTH),
        theme::fg(palette::PRIMARY),
    ));
    spans.push(Span::raw(" | "));
    spans.push(Span::styled(
        position.outcome_label.clone(),
        theme::fg(palette::TEXT),
    ));
    spans.push(Span::raw(" | "));
    spans.push(Span::styled(
        format!("{:.2} @ ${:.3}", position.shares, position.avg_price),
        theme::fg(palette::MUTED),
    ));
    spans.push(Span::raw(" | PnL "));

    // Realized PnL in positive/danger color by sign.
    let pnl_color = if position.realized_pnl >= 0.0 {
        palette::POSITIVE
    } else {
        palette::DANGER
    };
    spans.push(Span::styled(
        format!("${:+.2}", position.realized_pnl),
        theme::fg_bold(pnl_color),
    ));

    ListItem::new(Line::from(spans))
}
