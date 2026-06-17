//! Confirmation modals drawn on top of the active panel (quit, leave market).

// =========================================================================================================
// Imports
// =========================================================================================================

use ratatui::{
    prelude::*,
    widgets::{Clear, Paragraph, Wrap},
};

use crate::data::{ExecutionMode, OrderSide};
use crate::tui::app::{App, LeaveSelection, QuitSelection, TradeField};
use crate::tui::theme::{self, palette};

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

/// The buy/sell trade-entry form. A pointer (`>`) rests on one field at a time; `↑/↓`
/// move it, `Enter` acts on the focused field. The actual keyboard handling lives in
/// `App::handle_trade_entry`.
pub(super) fn draw_trade_entry_modal(frame: &mut Frame, area: Rect, app: &App) {
    let entry = &app.trade_entry;
    let market = app.watched_markets_info.get(entry.market_index);

    let market_name = market
        .map(|m| theme::truncate(&m.question, 44))
        .unwrap_or_else(|| "Unknown market".to_string());
    let outcome_label = market
        .and_then(|m| m.outcomes.get(entry.outcome_index))
        .cloned()
        .unwrap_or_else(|| format!("#{}", entry.outcome_index));

    // Side and mode value colors in their semantic palette.
    let side_color = match entry.side {
        OrderSide::Buy => palette::POSITIVE,
        OrderSide::Sell => palette::DANGER,
    };
    let mode_color = match entry.mode {
        ExecutionMode::Simulated => palette::ACCENT,
        ExecutionMode::Real => palette::POSITIVE,
    };
    let size_display = if entry.size_input.is_empty() {
        "0".to_string()
    } else {
        entry.size_input.clone()
    };

    // The pointer is shown on the focused field; while editing it becomes a caret hint.
    let pointer = |field: TradeField| -> Span<'static> {
        if entry.focus == field {
            let glyph = if entry.editing { "▸" } else { ">" };
            Span::styled(format!("{} ", glyph), theme::fg_bold(palette::SELECTED))
        } else {
            Span::raw("  ")
        }
    };

    // A value span that brightens when its field is being edited.
    let editing = |field: TradeField, color| {
        if entry.focus == field && entry.editing {
            theme::fg_bold(palette::SELECTED)
        } else {
            theme::fg_bold(color)
        }
    };

    let body = vec![
        Line::raw(""),
        Line::from(Span::styled(market_name, theme::fg(palette::PRIMARY))),
        Line::raw(""),
        Line::from(vec![
            pointer(TradeField::Outcome),
            Span::styled("Outcome:  ", theme::fg(palette::MUTED)),
            Span::styled(outcome_label, editing(TradeField::Outcome, palette::TEXT)),
        ]),
        Line::from(vec![
            pointer(TradeField::Side),
            Span::styled("Side:     ", theme::fg(palette::MUTED)),
            Span::styled(entry.side.as_str(), theme::fg_bold(side_color)),
        ]),
        Line::from(vec![
            pointer(TradeField::Mode),
            Span::styled("Mode:     ", theme::fg(palette::MUTED)),
            Span::styled(entry.mode.as_str(), theme::fg_bold(mode_color)),
        ]),
        Line::from(vec![
            pointer(TradeField::Size),
            Span::styled("Size:     ", theme::fg(palette::MUTED)),
            Span::styled(size_display, editing(TradeField::Size, palette::TEXT)),
        ]),
        Line::raw(""),
        Line::from(vec![
            pointer(TradeField::Submit),
            Span::styled(
                " Submit ",
                if entry.focus == TradeField::Submit {
                    theme::pill(palette::POSITIVE, palette::TEXT)
                } else {
                    theme::fg(palette::FAINT)
                },
            ),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            hint(entry),
            theme::fg(palette::FAINT),
        )),
    ];

    let width = 56u16;
    let height = (body.len() as u16) + 2;
    let modal_area = theme::centered_rect(width, height, area);

    frame.render_widget(Clear, modal_area);
    let modal = Paragraph::new(body)
        .block(theme::titled_block("💱 Trade Entry", palette::INFO))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(modal, modal_area);
}

/// Context-sensitive footer hint for the trade-entry form.
fn hint(entry: &crate::tui::app::TradeEntryState) -> &'static str {
    if entry.editing {
        match entry.focus {
            TradeField::Outcome => "[←/→] change   [Enter] confirm   [Esc] back",
            _ => "type size   [Enter] confirm   [Esc] back",
        }
    } else {
        "[↑/↓] move   [Enter] select   [Esc] cancel"
    }
}
