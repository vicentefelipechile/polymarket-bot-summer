//! Markets panels: the searchable market list (`draw_markets`) and the detailed view of a
//! watched market with its live detection analysis (`draw_market_detail`).

// =========================================================================================================
// Imports
// =========================================================================================================

use ratatui::{
    prelude::*,
    widgets::{List, ListItem, Paragraph, Wrap},
};

use crate::tui::app::App;
use crate::tui::theme::{self, palette};

// =========================================================================================================
// Market list
// =========================================================================================================

pub(super) fn draw_markets(frame: &mut Frame, area: Rect, app: &App) {
    // Split: Search info + Market list
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    // Search header
    let search_info = if app.is_loading_markets {
        Line::from(vec![Span::styled(
            "  Loading markets...",
            Style::default().fg(Color::Yellow),
        )])
    } else if !app.market_search_query.is_empty() {
        Line::from(vec![
            Span::raw("  Search: "),
            Span::styled(
                &app.market_search_query,
                Style::default().fg(Color::Cyan).bold(),
            ),
            Span::raw(" | "),
            Span::styled(
                format!("{} results", app.available_markets.len()),
                Style::default().fg(Color::Green),
            ),
            Span::raw(" | "),
            Span::styled(
                "↑↓ Navigate, Enter to join",
                Style::default().fg(Color::Gray),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("  Press ", Style::default().fg(Color::Gray)),
            Span::styled("S", Style::default().fg(Color::Yellow).bold()),
            Span::styled(" to search or ", Style::default().fg(Color::Gray)),
            Span::styled("T", Style::default().fg(Color::Yellow).bold()),
            Span::styled(" for trending markets", Style::default().fg(Color::Gray)),
        ])
    };

    let search_widget = Paragraph::new(search_info)
        .block(theme::titled_block("🔍 Market Search", palette::PRIMARY));

    frame.render_widget(search_widget, layout[0]);

    // Market list
    let items: Vec<ListItem> = if app.available_markets.is_empty() {
        vec![
            ListItem::new(Line::raw("")),
            ListItem::new(Line::styled(
                "  No markets loaded",
                Style::default().fg(Color::Yellow),
            )),
            ListItem::new(Line::raw("")),
            ListItem::new(Line::styled(
                "  Commands:",
                Style::default().fg(Color::Gray),
            )),
            ListItem::new(Line::styled(
                "    /search <keyword>  - Search markets",
                Style::default().fg(Color::Gray),
            )),
            ListItem::new(Line::styled(
                "    /trending          - Show trending",
                Style::default().fg(Color::Gray),
            )),
            ListItem::new(Line::styled(
                "    /joinmarket <#>    - Join by index",
                Style::default().fg(Color::Gray),
            )),
        ]
    } else {
        app.available_markets
            .iter()
            .enumerate()
            .map(|(i, market)| {
                let is_selected = i == app.selected_market_index;
                let is_joined = app.joined_markets.contains(&market.id);

                let prefix = if is_selected { "▶ " } else { "  " };
                let index_style = if is_selected {
                    Style::default().fg(Color::Yellow).bold()
                } else {
                    Style::default().fg(Color::Gray)
                };

                let question_style = if is_joined {
                    Style::default().fg(Color::Green)
                } else if is_selected {
                    Style::default().fg(Color::White).bold()
                } else {
                    Style::default().fg(Color::White)
                };

                let joined_marker = if is_joined { " ✓" } else { "" };

                // Truncate question to fit
                let max_len = 60;
                let question = if market.question.len() > max_len {
                    format!("{}...", &market.question[..max_len])
                } else {
                    market.question.clone()
                };

                // Price display
                let price_info = if market.prices.len() >= 2 {
                    format!(
                        " [{:.0}%/{:.0}%]",
                        market.prices[0] * 100.0,
                        market.prices[1] * 100.0
                    )
                } else if !market.prices.is_empty() {
                    format!(" [{:.0}%]", market.prices[0] * 100.0)
                } else {
                    String::new()
                };

                ListItem::new(Line::from(vec![
                    Span::raw(prefix),
                    Span::styled(format!("{:2}. ", i + 1), index_style),
                    Span::styled(question, question_style),
                    Span::styled(price_info, Style::default().fg(Color::Cyan)),
                    Span::styled(joined_marker, Style::default().fg(Color::Green)),
                ]))
            })
            .collect()
    };

    let markets_list = List::new(items).block(theme::titled_block(
        &format!("📈 Markets ({})", app.available_markets.len()),
        palette::SELECTED,
    ));

    frame.render_widget(markets_list, layout[1]);
}

// =========================================================================================================
// Market detail
// =========================================================================================================

pub(super) fn draw_market_detail(frame: &mut Frame, area: Rect, app: &App) {
    // If no watched markets, show message
    if app.watched_markets_info.is_empty() {
        let msg = Paragraph::new(vec![
            Line::raw(""),
            Line::styled(
                "  No markets being watched",
                Style::default().fg(Color::Yellow),
            ),
            Line::raw(""),
            Line::styled(
                "  Join a market from the ",
                Style::default().fg(Color::Gray),
            ),
            Line::from(vec![
                Span::styled("  ", Style::default().fg(Color::Gray)),
                Span::styled("Markets", Style::default().fg(Color::Cyan).bold()),
                Span::styled(" tab to view details", Style::default().fg(Color::Gray)),
            ]),
        ])
        .block(theme::titled_block("📊 Market Detail", palette::SELECTED));
        frame.render_widget(msg, area);
        return;
    }

    // Get current selected market
    let market_index = app
        .selected_watched_market_index
        .min(app.watched_markets_info.len().saturating_sub(1));
    let market = &app.watched_markets_info[market_index];

    // Split into 3 columns: List (left), Info (center), Analysis (right)
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(25),     // Market List
            Constraint::Percentage(30), // Market Info
            Constraint::Min(40),        // Analysis
        ])
        .split(area);

    // COLUMN 1: Market List
    let items: Vec<ListItem> = app
        .watched_markets_info
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let is_selected = i == app.selected_watched_market_index;
            let style = if is_selected {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default().fg(Color::Gray)
            };

            // Show simple name or ID
            let name = if m.question.len() > 18 {
                format!("{}...", &m.question[..18])
            } else {
                m.question.clone()
            };

            let prefix = if is_selected { "> " } else { "  " };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(name, style),
            ]))
        })
        .collect();

    let list = List::new(items).block(theme::titled_block("Markets", palette::INFO));
    frame.render_widget(list, columns[0]);

    // COLUMN 2: Market Information
    let mut info_lines = vec![
        Line::from(vec![Span::styled(
            "  Question: ",
            Style::default().fg(Color::Cyan).bold(),
        )]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(&market.question, Style::default().fg(Color::White)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Market ID: ", Style::default().fg(Color::Gray)),
            Span::styled(
                &market.id[..16.min(market.id.len())],
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("..."),
        ]),
        Line::raw(""),
    ];

    // Outcomes and prices
    if !market.outcomes.is_empty() {
        info_lines.push(Line::styled(
            "  Outcomes & Prices:",
            Style::default().fg(Color::Cyan).bold(),
        ));
        for (i, outcome) in market.outcomes.iter().enumerate() {
            let price = market.prices.get(i).unwrap_or(&0.0);
            let price_pct = price * 100.0;
            let color = if price_pct > 60.0 {
                Color::Green
            } else if price_pct > 40.0 {
                Color::Yellow
            } else {
                Color::Red
            };

            info_lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(outcome, Style::default().fg(Color::White)),
                Span::raw(": "),
                Span::styled(
                    format!("{:.1}%", price_pct),
                    Style::default().fg(color).bold(),
                ),
            ]));
        }
        info_lines.push(Line::raw(""));
    }

    // Volume
    info_lines.push(Line::from(vec![
        Span::styled("  Volume: ", Style::default().fg(Color::Gray)),
        Span::styled(&market.volume, Style::default().fg(Color::Cyan)),
    ]));

    // Status
    info_lines.push(Line::from(vec![
        Span::styled("  Status: ", Style::default().fg(Color::Gray)),
        if market.active {
            Span::styled("Active", Style::default().fg(Color::Green).bold())
        } else {
            Span::styled("Closed", Style::default().fg(Color::Red))
        },
    ]));

    let info_widget = Paragraph::new(info_lines)
        .block(theme::titled_block("📋 Market Info", palette::PRIMARY))
        .wrap(Wrap { trim: true });

    frame.render_widget(info_widget, columns[1]);

    // RIGHT COLUMN: Detection Analysis
    let analysis_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Volume velocity graph
            Constraint::Length(6),  // OBI visualization
            Constraint::Min(5),     // Recent events
        ])
        .split(columns[2]);

    // Get analysis data for this market (if available)
    let analysis = app.market_analysis_data.get(&market.id);

    // Volume Velocity Graph (ASCII)
    let mut velocity_lines = vec![
        Line::styled(
            "  Volume Velocity (V_v)",
            Style::default().fg(Color::Yellow).bold(),
        ),
        Line::raw(""),
    ];

    if let Some(analysis) = analysis {
        if let Some(velocity) = analysis.current_velocity {
            let velocity_str = format!("{:+.2}", velocity);
            let velocity_color = if velocity.abs() > 1000.0 {
                Color::Red
            } else if velocity.abs() > 500.0 {
                Color::Yellow
            } else {
                Color::Green
            };

            velocity_lines.push(Line::from(vec![
                Span::raw("  Current: "),
                Span::styled(velocity_str, Style::default().fg(velocity_color).bold()),
                Span::raw(" vol/sec"),
            ]));

            // Simple ASCII bar
            let bar_length = (velocity.abs() / 50.0).min(30.0) as usize;
            let bar = "█".repeat(bar_length);
            velocity_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(bar, Style::default().fg(velocity_color)),
            ]));
        } else {
            velocity_lines.push(Line::styled(
                "  No data yet",
                Style::default().fg(Color::Gray),
            ));
        }
    } else {
        velocity_lines.push(Line::styled(
            "  Collecting data...",
            Style::default().fg(Color::Gray),
        ));
    }

    velocity_lines.push(Line::raw(""));
    velocity_lines.push(Line::styled(
        "  Threshold: 1000.0 vol/sec",
        Style::default().fg(Color::Gray),
    ));

    let velocity_widget =
        Paragraph::new(velocity_lines).block(theme::titled_block("📈 Velocity", palette::SELECTED));

    frame.render_widget(velocity_widget, analysis_layout[0]);

    // OBI (Order Book Imbalance) Visualization
    let mut obi_lines = vec![
        Line::styled(
            "  Order Book Imbalance",
            Style::default().fg(Color::Magenta).bold(),
        ),
        Line::raw(""),
    ];

    if let Some(analysis) = analysis {
        if let Some(obi) = analysis.current_obi {
            let obi_pct = obi * 100.0;
            let obi_color = if obi.abs() > 0.3 {
                Color::Red
            } else {
                Color::Green
            };

            obi_lines.push(Line::from(vec![
                Span::raw("  OBI: "),
                Span::styled(
                    format!("{:+.2}%", obi_pct),
                    Style::default().fg(obi_color).bold(),
                ),
            ]));

            // Visual bar from -100% to +100%
            let bar_pos = ((obi + 1.0) / 2.0 * 30.0) as usize;
            let left = "─".repeat(bar_pos.min(30));
            let right = "─".repeat((30 - bar_pos).max(0));
            obi_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(left, Style::default().fg(Color::Red)),
                Span::styled("●", Style::default().fg(obi_color).bold()),
                Span::styled(right, Style::default().fg(Color::Green)),
            ]));
        } else {
            obi_lines.push(Line::styled(
                "  No data yet",
                Style::default().fg(Color::Gray),
            ));
        }
    } else {
        obi_lines.push(Line::styled(
            "  Collecting data...",
            Style::default().fg(Color::Gray),
        ));
    }

    let obi_widget =
        Paragraph::new(obi_lines).block(theme::titled_block("⚖️  OBI", palette::ACCENT));

    frame.render_widget(obi_widget, analysis_layout[1]);

    // Recent Spike Events
    let mut events_lines = vec![];

    if let Some(analysis) = analysis {
        if analysis.recent_events.is_empty() {
            events_lines.push(Line::styled(
                "  No spike events detected yet",
                Style::default().fg(Color::Gray),
            ));
        } else {
            for event in analysis.recent_events.iter().take(5) {
                let time = chrono::DateTime::from_timestamp(event.timestamp, 0)
                    .map(|dt| dt.format("%H:%M:%S").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                events_lines.push(Line::from(vec![
                    Span::styled(format!("  [{}] ", time), Style::default().fg(Color::Gray)),
                    Span::styled("Velocity: ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("{:+.1}", event.velocity),
                        Style::default().fg(Color::Red).bold(),
                    ),
                ]));
            }
        }
    } else {
        events_lines.push(Line::styled(
            "  Initializing detector...",
            Style::default().fg(Color::Gray),
        ));
    }

    let events_widget = Paragraph::new(events_lines)
        .block(theme::titled_block("🔔 Recent Events", palette::DANGER));

    frame.render_widget(events_widget, analysis_layout[2]);
}
