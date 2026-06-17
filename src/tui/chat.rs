//! Chat interface for the AI chatbot: state and rendering.

// =========================================================================================================
// Imports
// =========================================================================================================

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
    Frame,
};

use crate::ai::chatbot::PendingConfirmation;
use crate::tui::theme::{self, palette};

// =========================================================================================================
// State
// =========================================================================================================

/// Chat state.
pub struct ChatState {
    pub input: String,
    pub input_active: bool,
    pub waiting_for_ai: bool,
    pub pending_confirmation: Option<PendingConfirmation>,
    pub history: Vec<(String, String)>,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            input_active: false,
            waiting_for_ai: false,
            pending_confirmation: None,
            history: Vec::new(),
        }
    }

    pub fn handle_char(&mut self, c: char) {
        if self.input_active && self.pending_confirmation.is_none() {
            self.input.push(c);
        }
    }

    pub fn handle_backspace(&mut self) {
        if self.input_active && self.pending_confirmation.is_none() {
            self.input.pop();
        }
    }

    pub fn clear_input(&mut self) {
        self.input.clear();
    }

    pub fn toggle_confirmation(&mut self) {
        if let Some(ref mut conf) = self.pending_confirmation {
            conf.selected = !conf.selected;
        }
    }
}

impl Default for ChatState {
    fn default() -> Self {
        Self::new()
    }
}

// =========================================================================================================
// Rendering
// =========================================================================================================

/// Render the chat interface
pub fn render_chat(f: &mut Frame, area: Rect, chat_state: &ChatState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Chat history
            Constraint::Length(3), // Input box
        ])
        .split(area);

    // If there's a pending confirmation, show modal
    if chat_state.pending_confirmation.is_some() {
        render_confirmation_modal(f, area, chat_state);
        return;
    }

    // Render chat history
    render_chat_history(f, chunks[0], chat_state);

    // Render input box
    render_chat_input(f, chunks[1], chat_state);
}

/// Render chat history
fn render_chat_history(f: &mut Frame, area: Rect, chat_state: &ChatState) {
    let history = &chat_state.history;

    let items: Vec<ListItem> = history
        .iter()
        .rev()
        .take(20)
        .rev()
        .map(|(role, message)| {
            let style = if role == "user" {
                theme::fg(palette::PRIMARY)
            } else if role == "model" {
                theme::fg(palette::POSITIVE)
            } else {
                theme::fg(palette::SELECTED)
            };

            let prefix = if role == "user" {
                "💬 You"
            } else if role == "model" {
                "🤖 AI"
            } else {
                "🔧 Function"
            };

            let mut lines = vec![Line::from(Span::styled(
                format!("{}: ", prefix),
                style.add_modifier(Modifier::BOLD),
            ))];

            // Add message content
            for line in message.lines() {
                lines.push(Line::from(Span::styled(line.to_string(), style)));
            }

            lines.push(Line::from(""));
            ListItem::new(lines)
        })
        .collect();

    let list = List::new(items).block(theme::titled_block("💬 AI Chat", palette::PRIMARY));

    f.render_widget(list, area);
}

/// Render chat input box
fn render_chat_input(f: &mut Frame, area: Rect, chat_state: &ChatState) {
    let input_text = if chat_state.waiting_for_ai {
        "⏳ AI is thinking...".to_string()
    } else if chat_state.input_active {
        format!("{}_", chat_state.input)
    } else {
        chat_state.input.clone()
    };

    let title = if chat_state.input_active {
        " 📝 Type your message (ESC to cancel, ENTER to send) "
    } else {
        " Press ENTER to start typing "
    };

    let accent = if chat_state.input_active {
        palette::POSITIVE
    } else {
        palette::PRIMARY
    };

    let input = Paragraph::new(input_text)
        .style(theme::fg(palette::TEXT))
        .block(theme::titled_block(title, accent));

    f.render_widget(input, area);
}

/// Render confirmation modal for sensitive actions.
fn render_confirmation_modal(f: &mut Frame, area: Rect, chat_state: &ChatState) {
    if let Some(ref conf) = chat_state.pending_confirmation {
        let body = vec![
            Line::from(Span::styled(
                "Confirmation Required",
                theme::fg_bold(palette::SELECTED),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                conf.description.clone(),
                theme::fg(palette::TEXT),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                "← → to toggle, Enter to confirm",
                theme::fg(palette::MUTED),
            )),
        ];

        theme::confirm_modal(
            f,
            area,
            "⚠️  Confirm Action",
            palette::DANGER,
            body,
            conf.selected,
        );
    }
}
