//! TUI rendering. Render-only: reads `App` state and draws via `theme` helpers, never
//! mutates state or performs IO (that lives in `app.rs`). All styling goes through `theme`.

// =========================================================================================================
// Imports
// =========================================================================================================

use ratatui::{
    prelude::*,
    widgets::{List, ListItem, Paragraph, Tabs, Wrap},
};

use crate::tui::app::{App, InputMode, LeaveSelection, LogLevel, QuitSelection, Tab};
use crate::tui::theme::{self, palette};

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

    draw_header(frame, layout[0], app);
    draw_tabs(frame, layout[1], app);
    draw_content(frame, layout[2], app);

    if app.input_mode == InputMode::Command {
        draw_command_input(frame, layout[3], app);
        draw_footer(frame, layout[4], app);
    } else {
        draw_footer(frame, layout[3], app);
    }

    // Draw quit confirmation modal on top if active
    if app.input_mode == InputMode::QuitConfirmation {
        draw_quit_confirmation_modal(frame, area, app);
    }

    // Draw leave market confirmation modal on top if active
    if app.input_mode == InputMode::LeaveMarketConfirmation {
        draw_leave_confirmation_modal(frame, area, app);
    }
}

// =========================================================================================================
// Panels & tabs
// =========================================================================================================

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let status = if app.is_paused {
        Span::styled(
            " PAUSED ",
            Style::default().bg(Color::Red).fg(Color::White).bold(),
        )
    } else {
        Span::styled(
            " ACTIVE ",
            Style::default().bg(Color::Green).fg(Color::Black).bold(),
        )
    };

    let markets_count = app.joined_markets.len();
    let markets_info = if markets_count > 0 {
        Span::styled(
            format!(" [{} markets] ", markets_count),
            Style::default().fg(Color::Yellow),
        )
    } else {
        Span::raw("")
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled("🟢 ", Style::default().fg(Color::Green)),
        Span::styled(
            "Polymarket Bot Summer",
            Style::default().fg(Color::Cyan).bold(),
        ),
        Span::raw(" - "),
        status,
        markets_info,
    ]))
    .block(theme::titled_block("Bot Status", palette::PRIMARY));

    frame.render_widget(header, area);
}

fn draw_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let navigating = app.input_mode == InputMode::TabNavigation;

    let titles: Vec<Line> = Tab::all()
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let style = if navigating && *t == app.highlighted_tab {
                // Pending selection while navigating the tab bar (not yet activated).
                Style::default().fg(Color::Cyan).bold()
            } else if *t == app.current_tab {
                // Active tab whose content is currently shown.
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(format!(" [{}] {} ", i + 1, t.title())).style(style)
        })
        .collect();

    // The underline highlight follows the cursor while navigating, otherwise the active tab.
    let selected = if navigating {
        app.highlighted_tab as usize
    } else {
        app.current_tab as usize
    };

    let title = if navigating {
        "Navigation (Enter to open, Esc to cancel)"
    } else {
        "Navigation"
    };

    let tabs = Tabs::new(titles)
        .block(theme::titled_block(title, palette::PRIMARY))
        .highlight_style(theme::fg_bold(palette::SELECTED))
        .select(selected);

    frame.render_widget(tabs, area);
}

fn draw_content(frame: &mut Frame, area: Rect, app: &App) {
    match app.current_tab {
        Tab::Dashboard => draw_dashboard(frame, area, app),
        Tab::Orders => draw_orders(frame, area, app),
        Tab::Markets => draw_markets(frame, area, app),
        Tab::MarketDetail => draw_market_detail(frame, area, app),
        Tab::Logs => draw_logs(frame, area, app),
        Tab::Docs => draw_docs(frame, area, app),
        Tab::AiChat => draw_ai_chat(frame, area, app),
        Tab::Settings => app.settings_editor.render(frame, area),
    }
}

fn draw_command_input(frame: &mut Frame, area: Rect, app: &App) {
    let input = Paragraph::new(Line::from(vec![
        Span::styled("Command: ", Style::default().fg(Color::Cyan).bold()),
        Span::styled(&app.command_input, Style::default().fg(Color::White)),
        Span::styled("▌", Style::default().fg(Color::Yellow)),
    ]))
    .block(theme::titled_block(
        "📝 Command Mode (ESC to cancel)",
        palette::SELECTED,
    ));

    frame.render_widget(input, area);
}

fn draw_dashboard(frame: &mut Frame, area: Rect, app: &App) {
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
        vec![
            Line::from(vec![
                Span::raw("  USDC Balance: "),
                Span::styled(
                    format!("${:.2}", p.usdc_balance),
                    Style::default().fg(Color::Green),
                ),
            ]),
            Line::from(vec![
                Span::raw("  Total Value:  "),
                Span::styled(
                    format!("${:.2}", p.total_value),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::raw("  P&L: "),
                Span::styled(
                    format!("{:+.2}", p.realized_pnl + p.unrealized_pnl),
                    if p.realized_pnl + p.unrealized_pnl >= 0.0 {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Red)
                    },
                ),
            ]),
        ]
    } else {
        vec![Line::styled(
            "  Loading...",
            Style::default().fg(Color::Yellow),
        )]
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

    // Right column: System Status
    let status_text = vec![
        Line::from(vec![
            Span::raw("  Trading:   "),
            if app.is_paused {
                Span::styled("PAUSED", Style::default().fg(Color::Red).bold())
            } else {
                Span::styled("ACTIVE", Style::default().fg(Color::Green).bold())
            },
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::raw("  WebSocket: "),
            Span::styled("Connected", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::raw("  Latency:   "),
            Span::styled("42ms", Style::default().fg(Color::Yellow)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::raw("  Last Order: "),
            if let Some(ref id) = app.last_order_id {
                Span::styled(&id[..12.min(id.len())], Style::default().fg(Color::Cyan))
            } else {
                Span::styled("None", Style::default().fg(Color::Gray))
            },
        ]),
    ];

    let status_widget =
        Paragraph::new(status_text).block(theme::titled_block("📊 System Status", palette::INFO));

    frame.render_widget(status_widget, columns[1]);
}

fn draw_orders(frame: &mut Frame, area: Rect, app: &App) {
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

fn draw_markets(frame: &mut Frame, area: Rect, app: &App) {
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

fn draw_market_detail(frame: &mut Frame, area: Rect, app: &App) {
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

fn draw_logs(frame: &mut Frame, area: Rect, app: &App) {
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

fn draw_docs(frame: &mut Frame, area: Rect, app: &App) {
    const DOC_SECTIONS: [&str; 5] = [
        "📖 How to Use This Bot",
        "🎯 What is Polymarket?",
        "💹 Trading Mechanics",
        "📊 Spike Detection",
        "📚 References",
    ];

    // Always show two-column layout
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(40)])
        .split(area);

    // Left: Section list (always visible)
    let items: Vec<ListItem> = DOC_SECTIONS
        .iter()
        .enumerate()
        .map(|(i, title)| {
            let is_selected = i == app.docs_selected_section;
            let is_viewing = app.docs_viewing_content && is_selected;
            let style = if is_selected {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if is_viewing {
                "● "
            } else if is_selected {
                "▶ "
            } else {
                "  "
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(*title, style),
            ]))
        })
        .collect();

    let list_title = if app.docs_viewing_content {
        " 📚 Documentation (Reading) "
    } else {
        " 📚 Documentation "
    };

    let list = List::new(items).block(theme::titled_block(list_title, palette::PRIMARY));
    frame.render_widget(list, layout[0]);

    // Right: Content or Preview
    if app.docs_viewing_content {
        // Show full content with scroll
        let content = get_doc_content(app.docs_selected_section);
        let scroll = app.docs_scroll_offset;

        let content_widget = Paragraph::new(content)
            .block(theme::titled_block(
                DOC_SECTIONS[app.docs_selected_section],
                palette::SELECTED,
            ))
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));
        frame.render_widget(content_widget, layout[1]);
    } else {
        // Show preview
        let preview = get_doc_preview(app.docs_selected_section);
        let preview_widget = Paragraph::new(preview)
            .block(theme::titled_block("Preview", palette::MUTED))
            .wrap(Wrap { trim: false });
        frame.render_widget(preview_widget, layout[1]);
    }
}

// =========================================================================================================
// Documentation content
// =========================================================================================================

fn get_doc_preview(section: usize) -> Vec<Line<'static>> {
    match section {
        0 => vec![
            Line::styled(
                "  How to Use This Bot",
                Style::default().fg(Color::Yellow).bold(),
            ),
            Line::raw(""),
            Line::raw("  Learn the basics of navigating and"),
            Line::raw("  controlling the Polymarket Bot."),
            Line::raw(""),
            Line::styled(
                "  Press Enter to read more...",
                Style::default().fg(Color::Gray),
            ),
        ],
        1 => vec![
            Line::styled(
                "  What is Polymarket?",
                Style::default().fg(Color::Yellow).bold(),
            ),
            Line::raw(""),
            Line::raw("  Polymarket is a decentralized"),
            Line::raw("  prediction market platform where"),
            Line::raw("  users trade on event outcomes."),
            Line::raw(""),
            Line::styled(
                "  Press Enter to read more...",
                Style::default().fg(Color::Gray),
            ),
        ],
        2 => vec![
            Line::styled(
                "  Trading Mechanics",
                Style::default().fg(Color::Yellow).bold(),
            ),
            Line::raw(""),
            Line::raw("  Understanding shares, prices,"),
            Line::raw("  order books, and how to trade."),
            Line::raw(""),
            Line::styled(
                "  Press Enter to read more...",
                Style::default().fg(Color::Gray),
            ),
        ],
        3 => vec![
            Line::styled(
                "  Spike Detection",
                Style::default().fg(Color::Yellow).bold(),
            ),
            Line::raw(""),
            Line::raw("  How this bot detects volume"),
            Line::raw("  spikes and market movements."),
            Line::raw(""),
            Line::styled(
                "  Press Enter to read more...",
                Style::default().fg(Color::Gray),
            ),
        ],
        4 => vec![
            Line::styled("  References", Style::default().fg(Color::Yellow).bold()),
            Line::raw(""),
            Line::raw("  Sources and links to learn"),
            Line::raw("  more about prediction markets."),
            Line::raw(""),
            Line::styled(
                "  Press Enter to read more...",
                Style::default().fg(Color::Gray),
            ),
        ],
        _ => vec![],
    }
}

fn get_doc_content(section: usize) -> Vec<Line<'static>> {
    match section {
        0 => vec![
            // HOW TO USE THIS BOT
            Line::styled(
                "  ═══════════════════════════════════════════════",
                Style::default().fg(Color::Cyan),
            ),
            Line::styled(
                "  HOW TO USE THIS BOT",
                Style::default().fg(Color::Yellow).bold(),
            ),
            Line::styled(
                "  ═══════════════════════════════════════════════",
                Style::default().fg(Color::Cyan),
            ),
            Line::raw(""),
            Line::styled("  NAVIGATION", Style::default().fg(Color::Green).bold()),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  • Use Tab or ←/→ arrow keys to switch between tabs"),
            Line::raw("  • Press 1-6 to jump directly to a specific tab"),
            Line::raw("  • Use ↑/↓ arrow keys to navigate lists"),
            Line::raw(""),
            Line::styled("  TABS OVERVIEW", Style::default().fg(Color::Green).bold()),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  [1] Dashboard  - View portfolio and system status"),
            Line::raw("  [2] Orders     - See your active orders"),
            Line::raw("  [3] Markets    - Search and join markets to watch"),
            Line::raw("  [4] Detail     - Detailed view of watched markets"),
            Line::raw("  [5] Logs       - View application logs and events"),
            Line::raw("  [6] Docs       - This documentation"),
            Line::raw(""),
            Line::styled(
                "  SEARCHING MARKETS",
                Style::default().fg(Color::Green).bold(),
            ),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  • Press 'S' for quick search"),
            Line::raw("  • Press 'T' for trending markets"),
            Line::raw("  • Use ':' or '/' to enter command mode"),
            Line::raw("  • Commands: /search <keyword>, /trending, /help"),
            Line::raw(""),
            Line::styled(
                "  JOINING MARKETS",
                Style::default().fg(Color::Green).bold(),
            ),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  • In Markets tab, use ↑/↓ to select a market"),
            Line::raw("  • Press Enter to join the selected market"),
            Line::raw("  • Or use /joinmarket <number> command"),
            Line::raw(""),
            Line::styled("  BOT CONTROLS", Style::default().fg(Color::Green).bold()),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  • P - Pause the bot (stops trading)"),
            Line::raw("  • R - Resume the bot (enable trading)"),
            Line::raw("  • ! - PANIC MODE (cancel all orders immediately)"),
            Line::raw("  • Q - Quit the application"),
            Line::raw(""),
        ],
        1 => vec![
            // WHAT IS POLYMARKET
            Line::styled(
                "  ═══════════════════════════════════════════════",
                Style::default().fg(Color::Cyan),
            ),
            Line::styled(
                "  WHAT IS POLYMARKET?",
                Style::default().fg(Color::Yellow).bold(),
            ),
            Line::styled(
                "  ═══════════════════════════════════════════════",
                Style::default().fg(Color::Cyan),
            ),
            Line::raw(""),
            Line::styled("  OVERVIEW", Style::default().fg(Color::Green).bold()),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  Polymarket is a decentralized prediction market"),
            Line::raw("  platform where users can bet on the outcomes of"),
            Line::raw("  real-world events across politics, sports, crypto,"),
            Line::raw("  and current affairs."),
            Line::raw(""),
            Line::styled("  HOW IT WORKS", Style::default().fg(Color::Green).bold()),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  • Events are presented as YES/NO questions"),
            Line::raw("  • Users buy 'shares' representing potential outcomes"),
            Line::raw("  • Share prices range from $0.00 to $1.00 USDC"),
            Line::raw("  • Price reflects the market's perceived probability"),
            Line::raw(""),
            Line::raw("  Example: If 'YES' costs $0.70, the market believes"),
            Line::raw("  there's a 70% chance the event will happen."),
            Line::raw(""),
            Line::styled(
                "  KEY CONCEPT: COLLATERALIZATION",
                Style::default().fg(Color::Green).bold(),
            ),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  Each pair of YES + NO shares = $1.00 USDC"),
            Line::raw("  This means one side ALWAYS pays out $1.00"),
            Line::raw("  when the market resolves."),
            Line::raw(""),
            Line::styled(
                "  PEER-TO-PEER TRADING",
                Style::default().fg(Color::Green).bold(),
            ),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  Unlike traditional betting:"),
            Line::raw("  • You trade with other users, not 'the house'"),
            Line::raw("  • No bookmaker setting arbitrary odds"),
            Line::raw("  • No limits on successful traders"),
            Line::raw(""),
            Line::styled("  RESOLUTION", Style::default().fg(Color::Green).bold()),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  When an event concludes:"),
            Line::raw("  • The correct outcome shares pay $1.00 each"),
            Line::raw("  • The incorrect outcome shares become worthless"),
            Line::raw("  • You can sell shares anytime before resolution"),
            Line::raw(""),
        ],
        2 => vec![
            // TRADING MECHANICS
            Line::styled(
                "  ═══════════════════════════════════════════════",
                Style::default().fg(Color::Cyan),
            ),
            Line::styled(
                "  TRADING MECHANICS",
                Style::default().fg(Color::Yellow).bold(),
            ),
            Line::styled(
                "  ═══════════════════════════════════════════════",
                Style::default().fg(Color::Cyan),
            ),
            Line::raw(""),
            Line::styled(
                "  UNDERSTANDING SHARES",
                Style::default().fg(Color::Green).bold(),
            ),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  • Buy YES if you think an event is MORE likely"),
            Line::raw("    than the current price suggests"),
            Line::raw("  • Buy NO if you think it's LESS likely"),
            Line::raw("  • Example: YES at $0.18 → Event happens → $1.00"),
            Line::raw("    Profit: $0.82 per share (456% return!)"),
            Line::raw(""),
            Line::styled(
                "  ORDER BOOK (CLOB)",
                Style::default().fg(Color::Green).bold(),
            ),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  Polymarket uses a 'hybrid-decentralized CLOB':"),
            Line::raw("  • CLOB = Central Limit Order Book"),
            Line::raw("  • Orders are matched off-chain (fast)"),
            Line::raw("  • Settlement happens on-chain (secure)"),
            Line::raw(""),
            Line::styled("  ORDER TYPES", Style::default().fg(Color::Green).bold()),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  • Market Order: Buy/sell immediately at best price"),
            Line::raw("  • Limit Order: Set your own price, wait for match"),
            Line::raw(""),
            Line::styled("  LIQUIDITY", Style::default().fg(Color::Green).bold()),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  • Liquidity = how easily you can buy/sell"),
            Line::raw("  • High liquidity = small price impact"),
            Line::raw("  • Low liquidity = larger price swings"),
            Line::raw("  • Market makers provide liquidity by posting orders"),
            Line::raw(""),
            Line::styled("  SPREAD", Style::default().fg(Color::Green).bold()),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  • Spread = difference between best buy and sell price"),
            Line::raw("  • Tight spread = efficient market"),
            Line::raw("  • Wide spread = hidden cost for impatient trades"),
            Line::raw(""),
            Line::styled("  FEES", Style::default().fg(Color::Green).bold()),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  • Trading fee: ~4% on transactions"),
            Line::raw("  • No fees for deposits or withdrawals"),
            Line::raw("  • USDC is the native currency"),
            Line::raw(""),
        ],
        3 => vec![
            // SPIKE DETECTION
            Line::styled(
                "  ═══════════════════════════════════════════════",
                Style::default().fg(Color::Cyan),
            ),
            Line::styled(
                "  SPIKE DETECTION",
                Style::default().fg(Color::Yellow).bold(),
            ),
            Line::styled(
                "  ═══════════════════════════════════════════════",
                Style::default().fg(Color::Cyan),
            ),
            Line::raw(""),
            Line::styled(
                "  WHAT IS VOLUME VELOCITY?",
                Style::default().fg(Color::Green).bold(),
            ),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  Volume Velocity (V_v) measures how fast trading"),
            Line::raw("  volume is changing over time. It's calculated as:"),
            Line::raw(""),
            Line::raw("    V_v = ΔVolume / ΔTime (volume per second)"),
            Line::raw(""),
            Line::raw("  A sudden spike in velocity often indicates:"),
            Line::raw("  • Breaking news affecting the market"),
            Line::raw("  • Large institutional trades"),
            Line::raw("  • Market manipulation attempts"),
            Line::raw(""),
            Line::styled("  THRESHOLDS", Style::default().fg(Color::Green).bold()),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  Velocity levels:"),
            Line::styled(
                "    Normal: <500 vol/sec",
                Style::default().fg(Color::Green),
            ),
            Line::styled(
                "    Elevated: 500-1000 vol/sec",
                Style::default().fg(Color::Yellow),
            ),
            Line::styled(
                "    Spike Alert: >1000 vol/sec",
                Style::default().fg(Color::Red),
            ),
            Line::raw(""),
            Line::styled(
                "  ORDER BOOK IMBALANCE (OBI)",
                Style::default().fg(Color::Green).bold(),
            ),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  OBI measures the difference between buy and sell"),
            Line::raw("  pressure in the order book:"),
            Line::raw(""),
            Line::raw("    OBI = (Bids - Asks) / (Bids + Asks)"),
            Line::raw(""),
            Line::raw("  Range: -1.0 (all sells) to +1.0 (all buys)"),
            Line::styled(
                "    Balanced: -0.3 to +0.3",
                Style::default().fg(Color::Green),
            ),
            Line::styled("    Imbalanced: >|0.3|", Style::default().fg(Color::Red)),
            Line::raw(""),
            Line::styled(
                "  HOW THE BOT USES THIS",
                Style::default().fg(Color::Green).bold(),
            ),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  The bot monitors these metrics in real-time:"),
            Line::raw("  • Alerts on unusual velocity spikes"),
            Line::raw("  • Tracks order book imbalances"),
            Line::raw("  • Logs significant events for analysis"),
            Line::raw("  • Can pause trading during extreme volatility"),
            Line::raw(""),
        ],
        4 => vec![
            // REFERENCES
            Line::styled(
                "  ═══════════════════════════════════════════════",
                Style::default().fg(Color::Cyan),
            ),
            Line::styled(
                "  REFERENCES & RESOURCES",
                Style::default().fg(Color::Yellow).bold(),
            ),
            Line::styled(
                "  ═══════════════════════════════════════════════",
                Style::default().fg(Color::Cyan),
            ),
            Line::raw(""),
            Line::styled(
                "  OFFICIAL DOCUMENTATION",
                Style::default().fg(Color::Green).bold(),
            ),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  • Polymarket Learn: polymarket.com/learn"),
            Line::raw("  • Polymarket Docs: docs.polymarket.com"),
            Line::raw("  • CLOB API Docs: docs.polymarket.com/api"),
            Line::raw(""),
            Line::styled(
                "  ARTICLES & GUIDES",
                Style::default().fg(Color::Green).bold(),
            ),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  • 'A Beginner's Guide to Prediction Markets'"),
            Line::raw("    Source: phemex.com"),
            Line::raw(""),
            Line::raw("  • 'How Polymarket's CLOB Works'"),
            Line::raw("    Source: rocknblock.io"),
            Line::raw(""),
            Line::raw("  • 'Trading Strategies for Prediction Markets'"),
            Line::raw("    Source: medium.com/@polymarket"),
            Line::raw(""),
            Line::styled("  KEY CONCEPTS", Style::default().fg(Color::Green).bold()),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  • USDC: USD Coin, the stablecoin used for trading"),
            Line::raw("  • CLOB: Central Limit Order Book"),
            Line::raw("  • AMM: Automated Market Maker (legacy system)"),
            Line::raw("  • Slippage: Price change during order execution"),
            Line::raw("  • Resolution: When a market outcome is determined"),
            Line::raw(""),
            Line::styled(
                "  RESEARCH PAPERS",
                Style::default().fg(Color::Green).bold(),
            ),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  • 'Prediction Markets: Theory & Practice'"),
            Line::raw("    Arrow et al., Science (2008)"),
            Line::raw(""),
            Line::raw("  • 'The Wisdom of Crowds in Markets'"),
            Line::raw("    Surowiecki, Anchor Books (2005)"),
            Line::raw(""),
            Line::styled("  DISCLAIMER", Style::default().fg(Color::Red).bold()),
            Line::raw("  ─────────────────────────────────────────────────"),
            Line::raw("  This bot is for educational purposes only."),
            Line::raw("  Trading involves risk. Never invest more than"),
            Line::raw("  you can afford to lose. DYOR (Do Your Own Research)."),
            Line::raw(""),
        ],
        _ => vec![],
    }
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let shortcuts = if app.input_mode == InputMode::TabNavigation {
        Line::from(vec![
            Span::styled(" [←→]", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Move  "),
            Span::styled("[Enter]", Style::default().fg(Color::Green).bold()),
            Span::raw("Open  "),
            Span::styled("[Esc]", Style::default().fg(Color::Red).bold()),
            Span::raw("Cancel"),
        ])
    } else if app.current_tab == Tab::Markets {
        Line::from(vec![
            Span::styled(" [S]", Style::default().fg(Color::Yellow).bold()),
            Span::raw("earch  "),
            Span::styled("[T]", Style::default().fg(Color::Cyan).bold()),
            Span::raw("rending  "),
            Span::styled("[:]", Style::default().fg(Color::Magenta).bold()),
            Span::raw("Command  "),
            Span::styled("[↑↓]", Style::default().fg(Color::Blue).bold()),
            Span::raw("Nav  "),
            Span::styled("[Enter]", Style::default().fg(Color::Green).bold()),
            Span::raw("Join  "),
            Span::styled("[Q]", Style::default().fg(Color::Red).bold()),
            Span::raw("uit"),
        ])
    } else if app.current_tab == Tab::MarketDetail {
        Line::from(vec![
            Span::styled(" [↑↓]", Style::default().fg(Color::Blue).bold()),
            Span::raw("Navigate  "),
            Span::styled("[Del/⌫]", Style::default().fg(Color::Red).bold()),
            Span::raw("Leave  "),
            Span::styled("[S]", Style::default().fg(Color::Cyan).bold()),
            Span::raw("earch  "),
            Span::styled("[:]", Style::default().fg(Color::Magenta).bold()),
            Span::raw("Cmd  "),
            Span::styled("[Q]", Style::default().fg(Color::Red).bold()),
            Span::raw("uit"),
        ])
    } else if app.current_tab == Tab::Docs {
        if app.docs_viewing_content {
            Line::from(vec![
                Span::styled(" [↑↓]", Style::default().fg(Color::Blue).bold()),
                Span::raw("Scroll  "),
                Span::styled("[⌫/Esc]", Style::default().fg(Color::Yellow).bold()),
                Span::raw("Back  "),
                Span::styled("[1-6]", Style::default().fg(Color::Cyan).bold()),
                Span::raw("Tabs  "),
                Span::styled("[Q]", Style::default().fg(Color::Red).bold()),
                Span::raw("uit"),
            ])
        } else {
            Line::from(vec![
                Span::styled(" [↑↓]", Style::default().fg(Color::Blue).bold()),
                Span::raw("Select  "),
                Span::styled("[Enter]", Style::default().fg(Color::Green).bold()),
                Span::raw("View  "),
                Span::styled("[←→]", Style::default().fg(Color::Yellow).bold()),
                Span::raw("Tabs  "),
                Span::styled("[Q]", Style::default().fg(Color::Red).bold()),
                Span::raw("uit"),
            ])
        }
    } else {
        Line::from(vec![
            Span::styled(" [P]", Style::default().fg(Color::Yellow).bold()),
            Span::raw("ause  "),
            Span::styled("[R]", Style::default().fg(Color::Green).bold()),
            Span::raw("esume  "),
            Span::styled("[S]", Style::default().fg(Color::Cyan).bold()),
            Span::raw("earch  "),
            Span::styled("[:]", Style::default().fg(Color::Magenta).bold()),
            Span::raw("Cmd  "),
            Span::styled("[H]", Style::default().fg(Color::Blue).bold()),
            Span::raw("elp  "),
            Span::styled("[Q]", Style::default().fg(Color::Red).bold()),
            Span::raw("uit"),
        ])
    };

    let footer = Paragraph::new(shortcuts)
        .block(theme::titled_block("Shortcuts", palette::MUTED))
        .alignment(Alignment::Center);

    frame.render_widget(footer, area);
}

// =========================================================================================================
// Modals
// =========================================================================================================

fn draw_quit_confirmation_modal(frame: &mut Frame, area: Rect, app: &App) {
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

fn draw_leave_confirmation_modal(frame: &mut Frame, area: Rect, app: &App) {
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

fn draw_ai_chat(frame: &mut Frame, area: Rect, app: &App) {
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
