//! Logs panel: scrolling list of recent application log entries (newest first).

// =========================================================================================================
// Imports
// =========================================================================================================

use ratatui::{
    prelude::*,
    widgets::{List, ListItem},
};

use crate::tui::app::{App, LogLevel};
use crate::tui::theme::{self, palette};

// =========================================================================================================
// Rendering
// =========================================================================================================

pub(super) fn draw_logs(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .logs
        .iter()
        .rev() // Show newest first
        .take(50)
        .map(|log| {
            let (prefix, style) = match log.level {
                LogLevel::Info => ("ℹ️ ", Style::default().fg(Color::Cyan)),
                LogLevel::Warning => ("⚠️ ", Style::default().fg(Color::Yellow)),
                LogLevel::Error => ("❌", Style::default().fg(Color::Red)),
                LogLevel::Success => ("✅", Style::default().fg(Color::Green)),
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("[{}] ", log.timestamp),
                    Style::default().fg(Color::Gray),
                ),
                Span::raw(prefix),
                Span::styled(&log.message, style),
            ]))
        })
        .collect();

    let logs_list = List::new(items).block(theme::titled_block(
        &format!("📝 Logs ({})", app.logs.len()),
        palette::MUTED,
    ));

    frame.render_widget(logs_list, area);
}
