//! Chat interface for AI chatbot

use crate::ai::chatbot::PendingConfirmation;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Chat state
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
                Style::default().fg(Color::Cyan)
            } else if role == "model" {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Yellow)
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

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" 💬 AI Chat "),
    );

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

    let input = Paragraph::new(input_text)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if chat_state.input_active {
                    Color::Green
                } else {
                    Color::Cyan
                }))
                .title(title),
        );

    f.render_widget(input, area);
}

/// Render confirmation modal for sensitive actions
fn render_confirmation_modal(f: &mut Frame, area: Rect, chat_state: &ChatState) {
    if let Some(ref conf) = chat_state.pending_confirmation {
        // Create centered modal
        let modal_width = 60;
        let modal_height = 12;
        let x = (area.width.saturating_sub(modal_width)) / 2;
        let y = (area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect {
            x: area.x + x,
            y: area.y + y,
            width: modal_width,
            height: modal_height,
        };

        // Clear background
        let background = Block::default().style(Style::default().bg(Color::Black).fg(Color::White));
        f.render_widget(background, area);

        // Modal content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(3),    // Description
                Constraint::Length(3), // Buttons
            ])
            .split(modal_area);

        // Title
        let title = Paragraph::new("⚠️  Confirmation Required")
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Description
        let desc = Paragraph::new(conf.description.clone())
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT));
        f.render_widget(desc, chunks[1]);

        // Buttons
        let yes_style = if conf.selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };

        let no_style = if !conf.selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red)
        };

        let buttons = Paragraph::new(Line::from(vec![
            Span::raw("    "),
            Span::styled("[ YES ]", yes_style),
            Span::raw("        "),
            Span::styled("[ NO ]", no_style),
            Span::raw("    "),
        ]))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Use ← → to toggle, Enter to confirm "),
        );
        f.render_widget(buttons, chunks[2]);
    }
}
