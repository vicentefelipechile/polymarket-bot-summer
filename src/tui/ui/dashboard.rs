//! Dashboard panel: portfolio summary, monitored markets, and system status.

// =========================================================================================================
// Imports
// =========================================================================================================

use ratatui::{
    prelude::*,
    widgets::{Paragraph, Wrap},
};

use crate::tui::app::App;
use crate::tui::theme::{self, palette};

// =========================================================================================================
// Rendering
// =========================================================================================================

pub(super) fn draw_dashboard(frame: &mut Frame, area: Rect, app: &App) {
    // Split into two columns
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left column: Portfolio + Joined Markets
    let left_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(columns[0]);

    // Portfolio
    let portfolio_text = if let Some(ref p) = app.portfolio {
        let pnl = p.realized_pnl + p.unrealized_pnl;
        let pnl_color = if pnl >= 0.0 {
            palette::POSITIVE
        } else {
            palette::DANGER
        };
        vec![
            Line::from(vec![
                Span::raw("  USDC Balance: "),
                Span::styled(
                    format!("${:.2}", p.usdc_balance),
                    theme::fg(palette::POSITIVE),
                ),
            ]),
            Line::from(vec![
                Span::raw("  Total Value:  "),
                Span::styled(
                    format!("${:.2}", p.total_value),
                    theme::fg(palette::PRIMARY),
                ),
            ]),
            Line::from(vec![
                Span::raw("  P&L: "),
                Span::styled(format!("{:+.2}", pnl), theme::fg(pnl_color)),
            ]),
            Line::raw(""),
            // Separate virtual simulation wallet (paper trading), tracked independently.
            Line::from(vec![
                Span::styled(" SIMULATED ", theme::pill(palette::ACCENT, palette::TEXT)),
                Span::raw(" "),
                Span::styled(
                    format!("${:.2}", p.simulated_balance),
                    theme::fg(palette::ACCENT),
                ),
            ]),
            Line::from(vec![
                Span::raw("  Sim P&L: "),
                Span::styled(
                    format!("{:+.2}", p.simulated_realized_pnl),
                    theme::fg(if p.simulated_realized_pnl >= 0.0 {
                        palette::POSITIVE
                    } else {
                        palette::DANGER
                    }),
                ),
            ]),
            Line::raw(""),
            // The real numbers above come from a stubbed engine; flag it rather than imply funds.
            Line::styled("  real: demo — no live account", theme::fg(palette::FAINT)),
        ]
    } else {
        vec![Line::styled("  Loading...", theme::fg(palette::SELECTED))]
    };

    let portfolio_widget = Paragraph::new(portfolio_text)
        .block(theme::titled_block("💰 Portfolio", palette::POSITIVE))
        .wrap(Wrap { trim: true });

    frame.render_widget(portfolio_widget, left_layout[0]);

    // Joined Markets
    let joined_text: Vec<Line> = if app.joined_markets.is_empty() {
        vec![
            Line::styled("  No markets joined", Style::default().fg(Color::Yellow)),
            Line::raw(""),
            Line::styled(
                "  Press 'S' to search markets",
                Style::default().fg(Color::Gray),
            ),
        ]
    } else {
        app.joined_markets
            .iter()
            .enumerate()
            .map(|(i, m)| {
                Line::from(vec![
                    Span::styled(format!("  {}. ", i + 1), Style::default().fg(Color::Gray)),
                    Span::styled(&m[..16.min(m.len())], Style::default().fg(Color::Cyan)),
                    Span::raw("..."),
                ])
            })
            .collect()
    };

    let joined_widget = Paragraph::new(joined_text).block(theme::titled_block(
        &format!("🎯 Monitoring ({})", app.joined_markets.len()),
        palette::ACCENT,
    ));

    frame.render_widget(joined_widget, left_layout[1]);

    // Right column: System Status — every field is derived from real `App` state. No
    // invented numbers (the old "WebSocket: Connected / Latency: 42ms" were placeholders).
    let status_text = system_status_lines(app);

    let status_widget =
        Paragraph::new(status_text).block(theme::titled_block("📊 System Status", palette::INFO));

    frame.render_widget(status_widget, columns[1]);
}

// =========================================================================================================
// Helpers
// =========================================================================================================

/// Aggregate detection signals across all monitored markets into a one-line summary.
///
/// Returns `(markets_with_signal, peak_velocity, peak_obi, recent_event_count)`. Reads the
/// simulated `market_analysis_data`; once the live detector is wired in, the same shape holds.
fn detection_summary(app: &App) -> (usize, f64, f64, usize) {
    let mut markets_with_signal = 0usize;
    let mut peak_velocity = 0.0f64;
    let mut peak_obi = 0.0f64;
    let mut recent_events = 0usize;

    for analysis in app.market_analysis_data.values() {
        let velocity = analysis.current_velocity.unwrap_or(0.0);
        let obi = analysis.current_obi.unwrap_or(0.0);
        // A market is "signalling" when its velocity crosses the alert threshold.
        if velocity.abs() > 1000.0 {
            markets_with_signal += 1;
        }
        if velocity.abs() > peak_velocity.abs() {
            peak_velocity = velocity;
        }
        if obi.abs() > peak_obi.abs() {
            peak_obi = obi;
        }
        recent_events += analysis.recent_events.len();
    }

    (markets_with_signal, peak_velocity, peak_obi, recent_events)
}

/// Build the System Status body. Honest by construction: the trading integration is still
/// stubbed (see `trading/execution.rs`), so the panel says so rather than faking a live feed.
fn system_status_lines(app: &App) -> Vec<Line<'static>> {
    let (signalling, peak_velocity, peak_obi, recent_events) = detection_summary(app);
    let seconds_since_refresh = app.last_refresh.elapsed().as_secs();

    let mut lines = vec![
        // Trading engine state (real: driven by pause/resume).
        Line::from(vec![
            Span::raw("  Trading:    "),
            if app.is_paused {
                Span::styled("PAUSED", theme::fg_bold(palette::DANGER))
            } else {
                Span::styled("ACTIVE", theme::fg_bold(palette::POSITIVE))
            },
        ]),
        // Be explicit that orders do not reach the CLOB yet — no fake "Connected" feed.
        Line::from(vec![
            Span::raw("  Mode:       "),
            Span::styled(
                "DEMO (CLOB integration pending)",
                theme::fg(palette::SELECTED),
            ),
        ]),
        Line::from(vec![
            Span::raw("  API feed:   "),
            Span::styled("Not connected", theme::fg(palette::MUTED)),
        ]),
        Line::raw(""),
        // Live counts straight from App state.
        Line::from(vec![
            Span::raw("  Monitoring: "),
            Span::styled(
                format!("{} markets", app.watched_markets_info.len()),
                theme::fg(palette::ACCENT),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Open orders:"),
            Span::styled(
                format!(" {}", app.active_orders.len()),
                theme::fg(palette::PRIMARY),
            ),
        ]),
        Line::raw(""),
    ];

    // Detection block: real aggregates over the analysis data the bot tracks.
    lines.push(Line::styled(
        "  ── Detection ──",
        theme::fg_bold(palette::ACCENT),
    ));
    if app.watched_markets_info.is_empty() {
        lines.push(Line::styled(
            "  No markets to analyze",
            theme::fg(palette::MUTED),
        ));
    } else {
        let signal_color = if signalling > 0 {
            palette::DANGER
        } else {
            palette::POSITIVE
        };
        lines.push(Line::from(vec![
            Span::raw("  Signalling: "),
            Span::styled(
                format!("{}/{}", signalling, app.watched_markets_info.len()),
                theme::fg_bold(signal_color),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  Peak V_v:   "),
            Span::styled(
                format!("{:+.0} vol/s", peak_velocity),
                theme::fg(palette::SELECTED),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  Peak OBI:   "),
            Span::styled(format!("{:+.2}", peak_obi), theme::fg(palette::SELECTED)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  Events:     "),
            Span::styled(format!("{}", recent_events), theme::fg(palette::INFO)),
        ]));
    }

    lines.push(Line::raw(""));

    // Last order id (real).
    lines.push(Line::from(vec![
        Span::raw("  Last order: "),
        if let Some(ref id) = app.last_order_id {
            Span::styled(theme::truncate(id, 12), theme::fg(palette::PRIMARY))
        } else {
            Span::styled("None", theme::fg(palette::MUTED))
        },
    ]));
    // Real freshness indicator instead of an invented latency figure.
    lines.push(Line::from(vec![
        Span::raw("  Updated:    "),
        Span::styled(
            format!("{}s ago", seconds_since_refresh),
            theme::fg(palette::FAINT),
        ),
    ]));

    lines
}
