//! Password prompt screen for encrypted configuration

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Password prompt state
pub struct PasswordPrompt {
    password: String,
    error_message: Option<String>,
    is_first_time: bool,
    confirm_password: Option<String>,
    stage: PasswordStage,
}

enum PasswordStage {
    EnterPassword,
    ConfirmPassword,
}

impl PasswordPrompt {
    /// Create a new password prompt (for existing config)
    pub fn new() -> Self {
        Self {
            password: String::new(),
            error_message: None,
            is_first_time: false,
            confirm_password: None,
            stage: PasswordStage::EnterPassword,
        }
    }

    /// Create a first-time setup prompt (requires confirmation)
    pub fn new_first_time() -> Self {
        Self {
            password: String::new(),
            error_message: None,
            is_first_time: true,
            confirm_password: Some(String::new()),
            stage: PasswordStage::EnterPassword,
        }
    }

    /// Handle keyboard input
    pub fn handle_input(&mut self, key: KeyEvent) -> Option<String> {
        // CRITICAL FIX: Only process KeyPress events to avoid duplicates
        // crossterm sends both Press and Release events, we only want Press
        use crossterm::event::KeyEventKind;
        if key.kind != KeyEventKind::Press {
            return None;
        }

        match key.code {
            KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' => {
                // Ctrl+C to exit
                std::process::exit(0);
            }
            KeyCode::Char(c) => match self.stage {
                PasswordStage::EnterPassword => self.password.push(c),
                PasswordStage::ConfirmPassword => {
                    if let Some(ref mut confirm) = self.confirm_password {
                        confirm.push(c);
                    }
                }
            },
            KeyCode::Backspace => match self.stage {
                PasswordStage::EnterPassword => {
                    self.password.pop();
                }
                PasswordStage::ConfirmPassword => {
                    if let Some(ref mut confirm) = self.confirm_password {
                        confirm.pop();
                    }
                }
            },
            KeyCode::Enter => {
                if self.is_first_time {
                    match self.stage {
                        PasswordStage::EnterPassword => {
                            if self.password.len() < 8 {
                                self.error_message = Some(
                                    "La contraseña debe tener al menos 8 caracteres".to_string(),
                                );
                            } else {
                                self.stage = PasswordStage::ConfirmPassword;
                                self.error_message = None;
                            }
                        }
                        PasswordStage::ConfirmPassword => {
                            if let Some(ref confirm) = self.confirm_password {
                                if &self.password == confirm {
                                    return Some(self.password.clone());
                                } else {
                                    self.error_message =
                                        Some("Las contraseñas no coinciden".to_string());
                                    self.password.clear();
                                    if let Some(ref mut c) = self.confirm_password {
                                        c.clear();
                                    }
                                    self.stage = PasswordStage::EnterPassword;
                                }
                            }
                        }
                    }
                } else {
                    // Not first time, just return password
                    return Some(self.password.clone());
                }
            }
            KeyCode::Esc => {
                // Allow ESC to exit
                std::process::exit(0);
            }
            _ => {}
        }
        None
    }

    /// Set error message (e.g., incorrect password)
    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
        self.password.clear();
        if let Some(ref mut confirm) = self.confirm_password {
            confirm.clear();
        }
        self.stage = PasswordStage::EnterPassword;
    }

    /// Render the password prompt UI
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Length(10),
                Constraint::Percentage(30),
            ])
            .split(area);

        // Title
        let title_text = if self.is_first_time {
            "🔐 Configuración Inicial - Summer Bot"
        } else {
            "🔐 Ingrese su Contraseña"
        };

        let title = Paragraph::new(title_text)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);

        f.render_widget(title, chunks[0]);

        // Input box
        let prompt_text = match self.stage {
            PasswordStage::EnterPassword => "Contraseña: ",
            PasswordStage::ConfirmPassword => "Confirmar: ",
        };

        let masked_password = match self.stage {
            PasswordStage::EnterPassword => "*".repeat(self.password.len()),
            PasswordStage::ConfirmPassword => {
                if let Some(ref confirm) = self.confirm_password {
                    "*".repeat(confirm.len())
                } else {
                    String::new()
                }
            }
        };

        let mut lines = vec![Line::from(vec![
            Span::styled(prompt_text, Style::default().fg(Color::Yellow)),
            Span::raw(&masked_password),
            Span::styled("_", Style::default().fg(Color::DarkGray)),
        ])];

        if self.is_first_time && matches!(self.stage, PasswordStage::EnterPassword) {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Mínimo 8 caracteres",
                Style::default().fg(Color::DarkGray),
            )));
        }

        if let Some(ref error) = self.error_message {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                error.clone(),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "ESC: Salir | ENTER: Confirmar",
            Style::default().fg(Color::DarkGray),
        )));

        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Password ");

        let input_para = Paragraph::new(lines)
            .block(input_block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });

        f.render_widget(input_para, chunks[1]);
    }
}

impl Default for PasswordPrompt {
    fn default() -> Self {
        Self::new()
    }
}
