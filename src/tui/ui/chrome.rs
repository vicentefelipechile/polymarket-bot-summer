//! Persistent TUI chrome: the header bar, the tab bar, the command-input line, and the
//! footer shortcuts. Rendered around the active panel's content on every frame.

// =========================================================================================================
// Imports
// =========================================================================================================

use ratatui::{
    prelude::*,
    widgets::{Paragraph, Tabs},
};

use crate::tui::app::{App, InputMode, Tab};
use crate::tui::theme::{self, palette};

// =========================================================================================================
// Header & tabs
// =========================================================================================================

pub(super) fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
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

pub(super) fn draw_tabs(frame: &mut Frame, area: Rect, app: &App) {
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

// =========================================================================================================
// Command input & footer
// =========================================================================================================

pub(super) fn draw_command_input(frame: &mut Frame, area: Rect, app: &App) {
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

pub(super) fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
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
            Span::styled("[B]", Style::default().fg(Color::Green).bold()),
            Span::raw("uy/Sell  "),
            Span::styled("[Del/⌫]", Style::default().fg(Color::Red).bold()),
            Span::raw("Leave  "),
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
                Span::styled("[1-9]", Style::default().fg(Color::Cyan).bold()),
                Span::raw("Panels  "),
                Span::styled("[Q]", Style::default().fg(Color::Red).bold()),
                Span::raw("uit"),
            ])
        } else {
            Line::from(vec![
                Span::styled(" [↑↓]", Style::default().fg(Color::Blue).bold()),
                Span::raw("Select  "),
                Span::styled("[Enter]", Style::default().fg(Color::Green).bold()),
                Span::raw("View  "),
                Span::styled("[Esc]", Style::default().fg(Color::Yellow).bold()),
                Span::raw("Panels  "),
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
            Span::styled("[Esc]", Style::default().fg(Color::Yellow).bold()),
            Span::raw("Panels  "),
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
