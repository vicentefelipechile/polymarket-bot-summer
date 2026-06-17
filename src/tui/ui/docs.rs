//! Docs panel: a two-column documentation viewer (section list + preview/full content).
//! The section copy lives in `get_doc_preview` / `get_doc_content`.

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
// Constants
// =========================================================================================================

const DOC_SECTIONS: [&str; 5] = [
    "📖 How to Use This Bot",
    "🎯 What is Polymarket?",
    "💹 Trading Mechanics",
    "📊 Spike Detection",
    "📚 References",
];

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
            Line::raw("  • Press Esc to enter panel navigation, then ←/→"),
            Line::raw("  • Press 1-8 to jump directly to a specific panel"),
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

// =========================================================================================================
// Rendering
// =========================================================================================================

pub(super) fn draw_docs(frame: &mut Frame, area: Rect, app: &App) {
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
