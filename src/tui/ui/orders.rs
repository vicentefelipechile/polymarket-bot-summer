//! Orders panel: the list of orders (real and simulated), newest first.

// =========================================================================================================
// Imports
// =========================================================================================================

use ratatui::{
    prelude::*,
    widgets::{List, ListItem},
};

use crate::data::{ExecutionMode, OrderSide};
use crate::tui::app::App;
use crate::tui::theme::{self, palette};

// =========================================================================================================
// Constants
// =========================================================================================================

/// Max characters of an order id shown before truncation.
const ORDER_ID_WIDTH: usize = 12;

// =========================================================================================================
// Rendering
// =========================================================================================================

pub(super) fn draw_orders(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = if app.active_orders.is_empty() {
        vec![ListItem::new(Line::styled(
            "  No orders yet",
            theme::fg(palette::SELECTED),
        ))]
    } else {
        app.active_orders.iter().map(order_row).collect()
    };

    let orders_list = List::new(items).block(theme::titled_block(
        &format!("📋 Orders ({})", app.active_orders.len()),
        palette::INFO,
    ));

    frame.render_widget(orders_list, area);
}

/// Build one list row for an order: `[SIMULATED] <id> | SIDE | size @ $price`.
fn order_row(order: &crate::data::OrderInfo) -> ListItem<'static> {
    // BUY in positive color, SELL in danger color (semantic palette, never raw Color::*).
    let side_color = match OrderSide::parse_str(&order.side) {
        OrderSide::Buy => palette::POSITIVE,
        OrderSide::Sell => palette::DANGER,
    };

    let mut spans = vec![Span::raw("  ")];

    // Simulated orders carry an ACCENT pill so they read clearly beside real ones.
    if order.execution_mode == ExecutionMode::Simulated {
        spans.push(Span::styled(
            " SIMULATED ",
            theme::pill(palette::ACCENT, palette::TEXT),
        ));
        spans.push(Span::raw(" "));
    }

    spans.push(Span::styled(
        theme::truncate(&order.order_id, ORDER_ID_WIDTH),
        theme::fg(palette::PRIMARY),
    ));
    spans.push(Span::raw(" | "));
    spans.push(Span::styled(order.side.clone(), theme::fg(side_color)));
    spans.push(Span::raw(" | "));
    spans.push(Span::styled(
        format!("{} @ ${:.2}", order.size, order.price),
        theme::fg(palette::TEXT),
    ));
    spans.push(Span::raw("  "));
    spans.push(Span::styled(
        format!("[{}]", order.status),
        theme::fg(palette::MUTED),
    ));

    ListItem::new(Line::from(spans))
}
