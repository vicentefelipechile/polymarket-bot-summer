use crate::ai::AiPersonality;
use crate::crypto::SecureConfig;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Settings editor for viewing and modifying configuration
pub struct SettingsEditor {
    // Editable fields
    pub max_order_size: String,
    pub min_order_size: String,
    pub volume_velocity_threshold: String,
    pub obi_threshold: String,
    pub gemini_api_key: String,
    pub ai_enabled: bool,
    pub ai_personality: AiPersonality,

    // Read-only reference (for display)
    private_key_masked: String,
    database_path: String,

    // UI state
    current_field: usize,
    edit_mode: bool,
    error_message: Option<String>,
    success_message: Option<String>,
    save_requested: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsAction {
    None,
    RequestSave,
    CancelChanges,
}

impl SettingsEditor {
    /// Create editor from existing config
    pub fn from_config(config: &SecureConfig) -> Self {
        // Mask private key for display
        let private_key_masked = if config.private_key.len() > 10 {
            format!(
                "{}...{}",
                &config.private_key[..6],
                &config.private_key[config.private_key.len() - 4..]
            )
        } else {
            "****".to_string()
        };

        Self {
            max_order_size: config.max_order_size.to_string(),
            min_order_size: config.min_order_size.to_string(),
            volume_velocity_threshold: config.volume_velocity_threshold.to_string(),
            obi_threshold: config.obi_threshold.to_string(),
            gemini_api_key: config.gemini_api_key.clone().unwrap_or_default(),
            ai_enabled: config.ai_enabled,
            ai_personality: config.ai_personality,
            private_key_masked,
            database_path: config.database_path.clone(),
            current_field: 0,
            edit_mode: false,
            error_message: None,
            success_message: None,
            save_requested: false,
        }
    }

    /// Handle keyboard input
    pub fn handle_input(&mut self, key: KeyEvent) -> SettingsAction {
        // Only process Press events
        if key.kind != KeyEventKind::Press {
            return SettingsAction::None;
        }

        // Clear messages on new input
        self.error_message = None;
        self.success_message = None;

        // F5 to save
        if key.code == KeyCode::F(5) {
            if let Err(e) = self.validate_all() {
                self.error_message = Some(e);
                return SettingsAction::None;
            }
            self.save_requested = true;
            return SettingsAction::RequestSave;
        }

        // ESC to cancel changes or exit edit mode
        if key.code == KeyCode::Esc {
            if self.edit_mode {
                self.edit_mode = false;
            } else {
                return SettingsAction::CancelChanges;
            }
            return SettingsAction::None;
        }

        // Tab navigation (only when not editing)
        if key.code == KeyCode::Tab && !self.edit_mode {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                self.current_field = if self.current_field == 0 {
                    6 // Total editable fields - 1
                } else {
                    self.current_field - 1
                };
            } else {
                self.current_field = (self.current_field + 1) % 7; // 7 editable fields
            }
            return SettingsAction::None;
        }

        // Enter to toggle edit mode
        if key.code == KeyCode::Enter && !self.edit_mode {
            self.edit_mode = true;
            return SettingsAction::None;
        }

        // Handle editing
        if self.edit_mode {
            self.handle_edit_input(key);
        }

        SettingsAction::None
    }

    /// Check if currently in edit mode
    pub fn is_editing(&self) -> bool {
        self.edit_mode
    }

    /// Get current field index
    pub fn current_field(&self) -> usize {
        self.current_field
    }

    /// Move to previous field
    pub fn move_up(&mut self) {
        if self.current_field > 0 {
            self.current_field -= 1;
        }
    }

    /// Move to next field
    pub fn move_down(&mut self) {
        if self.current_field < 6 {
            self.current_field += 1;
        }
    }

    fn handle_edit_input(&mut self, key: KeyEvent) {
        match self.current_field {
            0..=3 => {
                // Numeric fields
                let field = match self.current_field {
                    0 => &mut self.max_order_size,
                    1 => &mut self.min_order_size,
                    2 => &mut self.volume_velocity_threshold,
                    3 => &mut self.obi_threshold,
                    _ => unreachable!(),
                };

                match key.code {
                    KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
                        field.push(c);
                    }
                    KeyCode::Backspace => {
                        field.pop();
                    }
                    KeyCode::Enter => {
                        self.edit_mode = false;
                    }
                    _ => {}
                }
            }
            4 => {
                // Gemini API key
                match key.code {
                    KeyCode::Char(c) => {
                        self.gemini_api_key.push(c);
                    }
                    KeyCode::Backspace => {
                        self.gemini_api_key.pop();
                    }
                    KeyCode::Enter => {
                        self.edit_mode = false;
                    }
                    _ => {}
                }
            }
            5 => {
                // AI enabled toggle
                if key.code == KeyCode::Char(' ') {
                    self.ai_enabled = !self.ai_enabled;
                    self.edit_mode = false;
                }
            }
            6 => {
                // AI personality - toggle between Summer and Anna
                if key.code == KeyCode::Char(' ') || key.code == KeyCode::Enter {
                    self.ai_personality = match self.ai_personality {
                        AiPersonality::Summer => AiPersonality::Anna,
                        AiPersonality::Anna => AiPersonality::Summer,
                    };
                    self.edit_mode = false;
                }
            }
            _ => {}
        }
    }

    fn validate_all(&self) -> Result<(), String> {
        let max: f64 = self
            .max_order_size
            .parse()
            .map_err(|_| "Max order size inválido".to_string())?;
        let min: f64 = self
            .min_order_size
            .parse()
            .map_err(|_| "Min order size inválido".to_string())?;
        let _vel: f64 = self
            .volume_velocity_threshold
            .parse()
            .map_err(|_| "Volume velocity inválido".to_string())?;
        let obi: f64 = self
            .obi_threshold
            .parse()
            .map_err(|_| "OBI threshold inválido".to_string())?;

        if max <= min {
            return Err("Max order debe ser mayor que min order".to_string());
        }

        if min < 0.1 {
            return Err("Min order debe ser al menos 0.1".to_string());
        }

        if obi < 0.1 || obi > 0.9 {
            return Err("OBI threshold debe estar entre 0.1 y 0.9".to_string());
        }

        if self.ai_enabled && self.gemini_api_key.is_empty() {
            return Err("API key requerida si AI está habilitado".to_string());
        }

        Ok(())
    }

    /// Create SecureConfig from current values
    pub fn to_config(&self, original: &SecureConfig) -> SecureConfig {
        SecureConfig {
            private_key: original.private_key.clone(), // Never change
            max_order_size: self
                .max_order_size
                .parse()
                .unwrap_or(original.max_order_size),
            min_order_size: self
                .min_order_size
                .parse()
                .unwrap_or(original.min_order_size),
            volume_velocity_threshold: self
                .volume_velocity_threshold
                .parse()
                .unwrap_or(original.volume_velocity_threshold),
            obi_threshold: self.obi_threshold.parse().unwrap_or(original.obi_threshold),
            database_path: original.database_path.clone(), // Never change
            rpc_url: original.rpc_url.clone(),
            gemini_api_key: if self.gemini_api_key.is_empty() {
                None
            } else {
                Some(self.gemini_api_key.clone())
            },
            ai_personality: self.ai_personality,
            ai_enabled: self.ai_enabled,
            ai_analysis_frequency_minutes: original.ai_analysis_frequency_minutes,
        }
    }

    pub fn set_success(&mut self, message: String) {
        self.success_message = Some(message);
        self.save_requested = false;
    }

    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
        self.save_requested = false;
    }

    pub fn is_save_requested(&self) -> bool {
        self.save_requested
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(10),   // Content
                Constraint::Length(3), // Footer
            ])
            .split(area);

        // Title
        let title = Paragraph::new("⚙️  Configuración")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        f.render_widget(title, chunks[0]);

        // Content
        let content_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8), // Trading params
                Constraint::Length(6), // AI settings
                Constraint::Length(5), // Security/Read-only
                Constraint::Min(1),    // Messages
            ])
            .split(chunks[1]);

        self.render_trading_params(f, content_chunks[0]);
        self.render_ai_settings(f, content_chunks[1]);
        self.render_readonly_fields(f, content_chunks[2]);
        self.render_messages(f, content_chunks[3]);

        // Footer
        let footer_text = if self.edit_mode {
            "[ENTER] Confirmar  [ESC] Cancelar"
        } else {
            "[TAB] Siguiente  [ENTER] Editar  [F5] Guardar  [ESC] Cancelar cambios"
        };

        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(footer, chunks[2]);
    }

    fn render_trading_params(&self, f: &mut Frame, area: Rect) {
        let fields = [
            ("Max Order Size (USDC):", &self.max_order_size, 0),
            ("Min Order Size (USDC):", &self.min_order_size, 1),
            (
                "Volume Velocity Threshold:",
                &self.volume_velocity_threshold,
                2,
            ),
            ("OBI Threshold:", &self.obi_threshold, 3),
        ];

        let mut lines = vec![
            Line::from(Span::styled(
                "Trading Parameters:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "────────────────────────────",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        for (label, value, field_idx) in fields {
            let is_current = self.current_field == field_idx;
            let color = if is_current && self.edit_mode {
                Color::Green
            } else if is_current {
                Color::Cyan
            } else {
                Color::White
            };

            let cursor = if is_current && self.edit_mode {
                "_"
            } else {
                ""
            };

            lines.push(Line::from(vec![
                Span::raw(format!("{:<28}", label)),
                Span::styled(value.clone(), Style::default().fg(color)),
                Span::styled(cursor, Style::default().fg(Color::Cyan)),
            ]));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let para = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(para, area);
    }

    fn render_ai_settings(&self, f: &mut Frame, area: Rect) {
        let is_toggle_current = self.current_field == 5;
        let is_personality_current = self.current_field == 6;
        let is_api_current = self.current_field == 4;

        let checkbox = if self.ai_enabled { "[✓]" } else { "[ ]" };
        let masked_api = if self.gemini_api_key.len() > 10 {
            format!("{}...", &self.gemini_api_key[..10])
        } else if self.gemini_api_key.is_empty() {
            "(vacío)".to_string()
        } else {
            self.gemini_api_key.clone()
        };

        let lines = vec![
            Line::from(Span::styled(
                "AI Settings:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "────────────────────────────",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(vec![
                Span::styled(
                    checkbox,
                    Style::default().fg(if is_toggle_current {
                        Color::Cyan
                    } else {
                        Color::White
                    }),
                ),
                Span::raw(" AI Enabled"),
            ]),
            Line::from(vec![
                Span::raw("Gemini API Key:              "),
                Span::styled(
                    masked_api,
                    Style::default().fg(if is_api_current {
                        if self.edit_mode {
                            Color::Green
                        } else {
                            Color::Cyan
                        }
                    } else {
                        Color::White
                    }),
                ),
                Span::styled(
                    if is_api_current && self.edit_mode {
                        "_"
                    } else {
                        ""
                    },
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(vec![
                Span::raw("Personality:                 "),
                Span::styled(
                    format!("{:?}", self.ai_personality),
                    Style::default().fg(if is_personality_current {
                        Color::Cyan
                    } else {
                        Color::White
                    }),
                ),
            ]),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let para = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(para, area);
    }

    fn render_readonly_fields(&self, f: &mut Frame, area: Rect) {
        let lines = vec![
            Line::from(Span::styled(
                "Security (Solo Lectura):",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "────────────────────────────",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(vec![
                Span::raw("Private Key:                 "),
                Span::styled(
                    &self.private_key_masked,
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(" 🔒", Style::default().fg(Color::Red)),
            ]),
            Line::from(vec![
                Span::raw("Database Path:               "),
                Span::styled(&self.database_path, Style::default().fg(Color::DarkGray)),
            ]),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let para = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(para, area);
    }

    fn render_messages(&self, f: &mut Frame, area: Rect) {
        let mut lines = vec![];

        if let Some(ref error) = self.error_message {
            lines.push(Line::from(Span::styled(
                format!("❌ {}", error),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
        }

        if let Some(ref success) = self.success_message {
            lines.push(Line::from(Span::styled(
                format!("✓ {}", success),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )));
        }

        if !lines.is_empty() {
            let para = Paragraph::new(lines)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: false });
            f.render_widget(para, area);
        }
    }
}
