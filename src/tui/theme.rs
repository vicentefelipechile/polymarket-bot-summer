//! Central theme and rendering helpers for the entire TUI.
//!
//! This module is the **single source of truth** for how the TUI looks: colors, block
//! borders, selection highlighting, modals, key hints, and text truncation. Every screen
//! (`ui`, `chat`, `settings`, `config_wizard`, `password_prompt`) must build its widgets
//! through these helpers instead of hand-picking `Color::*` / `Block::default()` inline.
//!
//! Why: when each screen invents its own styling, new screens drift visually and break the
//! shared look. Centralizing here means a color or border change happens in one place and
//! every screen follows.

// =========================================================================================================
// Imports
// =========================================================================================================

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

// =========================================================================================================
// Semantic palette
// =========================================================================================================

/// Semantic colors. Use these names, never a raw `Color::*`, so meaning stays consistent.
pub mod palette {
    use ratatui::style::Color;

    /// Branding, titles, primary borders, focused-but-not-editing fields.
    pub const PRIMARY: Color = Color::Cyan;
    /// Selection, section headers, non-blocking warnings.
    pub const SELECTED: Color = Color::Yellow;
    /// Positive values, success, active state, in-progress editing.
    pub const POSITIVE: Color = Color::Green;
    /// Negative values, danger, errors, secrets.
    pub const DANGER: Color = Color::Red;
    /// Secondary accent (e.g. monitoring, OBI).
    pub const ACCENT: Color = Color::Magenta;
    /// Informational accent (e.g. system status, navigation).
    pub const INFO: Color = Color::Blue;
    /// Primary foreground text.
    pub const TEXT: Color = Color::White;
    /// De-emphasized text, hints, cursors, disabled fields.
    pub const MUTED: Color = Color::Gray;
    /// Most de-emphasized text (sub-hints, masked values).
    pub const FAINT: Color = Color::DarkGray;
}

// =========================================================================================================
// Style constructors
// =========================================================================================================

/// Plain foreground style in a semantic color.
pub fn fg(color: Color) -> Style {
    Style::default().fg(color)
}

/// Bold foreground style in a semantic color.
pub fn fg_bold(color: Color) -> Style {
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

/// A highlighted "pill" style (background fill), e.g. selected buttons or status badges.
pub fn pill(bg: Color, fg: Color) -> Style {
    Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD)
}

// =========================================================================================================
// Blocks
// =========================================================================================================

/// A bordered block with a title, in the given accent color. This is THE way to wrap a
/// widget in a panel — do not build `Block::default().borders(...).title(...)` inline.
pub fn titled_block(title: &str, accent: Color) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(fg(accent))
        .title(format!(" {} ", title.trim()))
}

/// A bordered block with no title, in the given accent color.
pub fn plain_block(accent: Color) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(fg(accent))
}

// =========================================================================================================
// Selection & list items
// =========================================================================================================

/// Style for a list row given whether it is the selected row.
pub fn row_style(selected: bool) -> Style {
    if selected {
        fg_bold(palette::SELECTED)
    } else {
        fg(palette::MUTED)
    }
}

/// The leading marker for a selectable row (`"> "` when selected, two spaces otherwise).
pub fn row_marker(selected: bool) -> &'static str {
    if selected {
        "> "
    } else {
        "  "
    }
}

/// Build a standard selectable line: marker + text, styled by selection state.
pub fn selectable_line(text: &str, selected: bool) -> Line<'static> {
    let style = row_style(selected);
    Line::from(vec![
        Span::styled(row_marker(selected), style),
        Span::styled(text.to_string(), style),
    ])
}

// =========================================================================================================
// Key hints (footers)
// =========================================================================================================

/// One `[Key]Label` hint pair for a footer/shortcut bar. Compose several into a `Line`.
pub fn key_hint(key: &str, label: &str, accent: Color) -> Vec<Span<'static>> {
    vec![
        Span::styled(format!("[{}]", key), fg_bold(accent)),
        Span::raw(format!("{}  ", label)),
    ]
}

// =========================================================================================================
// Modals
// =========================================================================================================

/// Compute a centered `Rect` of the given size inside `area`, clamped to fit.
pub fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

/// Render a centered confirmation modal with a Yes/No choice.
///
/// `body` lines are shown above the buttons; `yes_selected` highlights the active choice.
/// This is the one way to draw a confirmation dialog — every screen shares its look.
pub fn confirm_modal(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    accent: Color,
    body: Vec<Line<'static>>,
    yes_selected: bool,
) {
    let width = 60u16;
    let height = (body.len() as u16) + 6;
    let modal_area = centered_rect(width, height, area);

    // Clear whatever is underneath the modal.
    frame.render_widget(Clear, modal_area);

    let yes_style = if yes_selected {
        pill(palette::DANGER, palette::TEXT)
    } else {
        fg(palette::MUTED)
    };
    let no_style = if yes_selected {
        fg(palette::MUTED)
    } else {
        pill(palette::POSITIVE, Color::Black)
    };

    let mut content = vec![Line::raw("")];
    content.extend(body);
    content.push(Line::raw(""));
    content.push(Line::from(vec![
        Span::raw("      "),
        Span::styled("  Yes  ", yes_style),
        Span::raw("    "),
        Span::styled("  No  ", no_style),
    ]));
    content.push(Line::raw(""));

    let modal = Paragraph::new(content)
        .block(titled_block(title, accent))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });

    frame.render_widget(modal, modal_area);
}

// =========================================================================================================
// Text helpers
// =========================================================================================================

/// Truncate a string to `max` characters, appending `…` when cut. Replaces the scattered
/// `&s[..n.min(s.len())]` slicing (which also panics on non-char-boundary byte indices).
pub fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        let kept: String = text.chars().take(max).collect();
        format!("{}…", kept)
    }
}
