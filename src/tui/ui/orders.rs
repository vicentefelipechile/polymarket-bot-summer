//! Orders panel: the list of currently active orders.

// =========================================================================================================
// Imports
// =========================================================================================================

use ratatui::{
    prelude::*,
    widgets::{List, ListItem},
};

use crate::tui::app::App;
use crate::tui::theme::{self, palette};

// =========================================================================================================
// Rendering
// =========================================================================================================

pub(super) fn draw_orders(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = if app.active_orders.is_empty() {
        vec![ListItem::new(Line::styled(
            "  No active orders",
            Style::default().fg(Color::Yellow),
        ))]
    } else {
        app.active_orders
            .iter()
            .map(|order| {
                let side_style = if order.side == "BUY" {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                };

                ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        &order.order_id[..12.min(order.order_id.len())],
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw(" | "),
                    Span::styled(&order.side, side_style),
                    Span::raw(" | "),
                    Span::raw(format!("{} @ ${:.2}", order.size, order.price)),
                ]))
            })
            .collect()
    };

    let orders_list = List::new(items).block(theme::titled_block(
        &format!("📋 Active Orders ({})", app.active_orders.len()),
        palette::INFO,
    ));

    frame.render_widget(orders_list, area);
}
