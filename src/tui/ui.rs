use crate::tui::app::{App, InputMode, LogLevel, Tab};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap},
};

/// Draw the complete TUI
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
}

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
            "Polymarket HFT Bot",
            Style::default().fg(Color::Cyan).bold(),
        ),
        Span::raw(" - "),
        status,
        markets_info,
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Bot Status "),
    );

    frame.render_widget(header, area);
}

fn draw_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let titles: Vec<Line> = Tab::all()
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let style = if *t == app.current_tab {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(format!(" [{}] {} ", i + 1, t.title())).style(style)
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" Navigation "))
        .highlight_style(Style::default().fg(Color::Yellow).bold())
        .select(app.current_tab as usize);

    frame.render_widget(tabs, area);
}

fn draw_content(frame: &mut Frame, area: Rect, app: &App) {
    match app.current_tab {
        Tab::Dashboard => draw_dashboard(frame, area, app),
        Tab::Orders => draw_orders(frame, area, app),
        Tab::Markets => draw_markets(frame, area, app),
        Tab::Logs => draw_logs(frame, area, app),
    }
}

fn draw_command_input(frame: &mut Frame, area: Rect, app: &App) {
    let input = Paragraph::new(Line::from(vec![
        Span::styled("Command: ", Style::default().fg(Color::Cyan).bold()),
        Span::styled(&app.command_input, Style::default().fg(Color::White)),
        Span::styled("▌", Style::default().fg(Color::Yellow)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" 📝 Command Mode (ESC to cancel) "),
    );

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
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 💰 Portfolio ")
                .border_style(Style::default().fg(Color::Green)),
        )
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

    let joined_widget = Paragraph::new(joined_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" 🎯 Monitoring ({}) ", app.joined_markets.len()))
            .border_style(Style::default().fg(Color::Magenta)),
    );

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

    let status_widget = Paragraph::new(status_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 📊 System Status ")
            .border_style(Style::default().fg(Color::Blue)),
    );

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

    let orders_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" 📋 Active Orders ({}) ", app.active_orders.len()))
            .border_style(Style::default().fg(Color::Blue)),
    );

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

    let search_widget = Paragraph::new(search_info).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 🔍 Market Search ")
            .border_style(Style::default().fg(Color::Cyan)),
    );

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

    let markets_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" 📈 Markets ({}) ", app.available_markets.len()))
            .border_style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(markets_list, layout[1]);
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

    let logs_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" 📝 Logs ({}) ", app.logs.len()))
            .border_style(Style::default().fg(Color::Gray)),
    );

    frame.render_widget(logs_list, area);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let shortcuts = if app.current_tab == Tab::Markets {
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
        .block(Block::default().borders(Borders::ALL).title(" Shortcuts "))
        .alignment(Alignment::Center);

    frame.render_widget(footer, area);
}
