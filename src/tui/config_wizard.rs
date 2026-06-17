//! Configuration wizard for first-time setup.

// =========================================================================================================
// Imports
// =========================================================================================================

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::ai::AiPersonality;
use crate::config::SecureConfig;
use crate::tui::theme::{self, palette};

// =========================================================================================================
// Types
// =========================================================================================================

/// Configuration wizard for first-time setup.
pub struct ConfigWizard {
    step: WizardStep,

    // Step 1: Password
    password: String,
    confirm_password: String,

    // Step 2: Private Key
    private_key: String,

    // Step 3: AI Settings
    gemini_api_key: String,
    ai_enabled: bool,
    ai_personality: AiPersonality,

    // Step 4: Trading Params
    max_order_size: String,
    min_order_size: String,
    volume_velocity: String,
    obi_threshold: String,

    // UI State
    current_field: usize,
    error_message: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum WizardStep {
    Password,
    PrivateKey,
    GeminiAPI,
    TradingParams,
    Confirmation,
}

// =========================================================================================================
// Implementation
// =========================================================================================================

impl ConfigWizard {
    pub fn new() -> Self {
        Self {
            step: WizardStep::Password,
            password: String::new(),
            confirm_password: String::new(),
            private_key: String::new(),
            gemini_api_key: String::new(),
            ai_enabled: false,
            ai_personality: AiPersonality::Summer,
            max_order_size: "100.0".to_string(),
            min_order_size: "1.0".to_string(),
            volume_velocity: "1000.0".to_string(),
            obi_threshold: "0.3".to_string(),
            current_field: 0,
            error_message: None,
        }
    }

    /// Handle keyboard input - returns Some(SecureConfig) when wizard completes
    pub fn handle_input(&mut self, key: KeyEvent) -> Option<SecureConfig> {
        // Only process Press events to avoid duplicates
        if key.kind != KeyEventKind::Press {
            return None;
        }

        // Global shortcuts
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            std::process::exit(0);
        }

        if key.code == KeyCode::Esc {
            std::process::exit(0);
        }

        match self.step {
            WizardStep::Password => self.handle_password_step(key),
            WizardStep::PrivateKey => self.handle_private_key_step(key),
            WizardStep::GeminiAPI => self.handle_gemini_step(key),
            WizardStep::TradingParams => self.handle_trading_params_step(key),
            WizardStep::Confirmation => self.handle_confirmation_step(key),
        }
    }

    fn handle_password_step(&mut self, key: KeyEvent) -> Option<SecureConfig> {
        match key.code {
            KeyCode::Char(c) => {
                if self.current_field == 0 {
                    self.password.push(c);
                } else {
                    self.confirm_password.push(c);
                }
            }
            KeyCode::Backspace => {
                if self.current_field == 0 {
                    self.password.pop();
                } else {
                    self.confirm_password.pop();
                }
            }
            KeyCode::Tab => {
                self.current_field = (self.current_field + 1) % 2;
            }
            KeyCode::Enter => {
                if self.password.len() < 8 {
                    self.error_message =
                        Some("La contraseña debe tener al menos 8 caracteres".to_string());
                } else if self.password != self.confirm_password {
                    self.error_message = Some("Las contraseñas no coinciden".to_string());
                } else {
                    self.error_message = None;
                    self.step = WizardStep::PrivateKey;
                    self.current_field = 0;
                }
            }
            _ => {}
        }
        None
    }

    fn handle_private_key_step(&mut self, key: KeyEvent) -> Option<SecureConfig> {
        match key.code {
            KeyCode::Char(c) => {
                self.private_key.push(c);
                self.error_message = None;
            }
            KeyCode::Backspace => {
                self.private_key.pop();
            }
            KeyCode::Left if key.modifiers.contains(KeyModifiers::NONE) => {
                // Go back to previous step
                self.step = WizardStep::Password;
                self.error_message = None;
            }
            KeyCode::Enter => {
                if !self.private_key.starts_with("0x") {
                    self.error_message = Some("Private key debe comenzar con 0x".to_string());
                } else if self.private_key.len() != 66 {
                    self.error_message =
                        Some("Private key debe tener 66 caracteres (0x + 64 hex)".to_string());
                } else if !self.private_key[2..].chars().all(|c| c.is_ascii_hexdigit()) {
                    self.error_message =
                        Some("Private key debe contener solo caracteres hexadecimales".to_string());
                } else {
                    self.error_message = None;
                    self.step = WizardStep::GeminiAPI;
                    self.current_field = 0;
                }
            }
            _ => {}
        }
        None
    }

    fn handle_gemini_step(&mut self, key: KeyEvent) -> Option<SecureConfig> {
        match key.code {
            KeyCode::Char(' ') => {
                // Toggle AI enabled
                self.ai_enabled = !self.ai_enabled;
            }
            KeyCode::Char(c) => {
                // Add to API key (only on first field)
                if self.current_field == 0 {
                    self.gemini_api_key.push(c);
                }
            }
            KeyCode::Backspace => {
                if self.current_field == 0 {
                    self.gemini_api_key.pop();
                }
            }
            KeyCode::Tab => {
                self.current_field = (self.current_field + 1) % 2;
            }
            KeyCode::Left if key.modifiers.contains(KeyModifiers::NONE) => {
                self.step = WizardStep::PrivateKey;
                self.error_message = None;
            }
            KeyCode::Enter => {
                if self.ai_enabled && self.gemini_api_key.is_empty() {
                    self.error_message =
                        Some("API key requerida si AI está habilitado".to_string());
                } else {
                    self.error_message = None;
                    self.step = WizardStep::TradingParams;
                    self.current_field = 0;
                }
            }
            _ => {}
        }
        None
    }

    fn handle_trading_params_step(&mut self, key: KeyEvent) -> Option<SecureConfig> {
        match key.code {
            KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => match self.current_field {
                0 => self.max_order_size.push(c),
                1 => self.min_order_size.push(c),
                2 => self.volume_velocity.push(c),
                3 => self.obi_threshold.push(c),
                _ => {}
            },
            KeyCode::Backspace => match self.current_field {
                0 => {
                    self.max_order_size.pop();
                }
                1 => {
                    self.min_order_size.pop();
                }
                2 => {
                    self.volume_velocity.pop();
                }
                3 => {
                    self.obi_threshold.pop();
                }
                _ => {}
            },
            KeyCode::Tab => {
                self.current_field = (self.current_field + 1) % 4;
            }
            KeyCode::Left if key.modifiers.contains(KeyModifiers::NONE) => {
                self.step = WizardStep::GeminiAPI;
                self.error_message = None;
            }
            KeyCode::Enter => {
                // Validate all params
                if let Err(e) = self.validate_trading_params() {
                    self.error_message = Some(e);
                } else {
                    self.error_message = None;
                    self.step = WizardStep::Confirmation;
                }
            }
            _ => {}
        }
        None
    }

    fn handle_confirmation_step(&mut self, key: KeyEvent) -> Option<SecureConfig> {
        match key.code {
            KeyCode::Left => {
                self.step = WizardStep::TradingParams;
                self.error_message = None;
            }
            KeyCode::Enter => {
                // Build and return config
                return Some(SecureConfig {
                    private_key: self.private_key.clone(),
                    max_order_size: self.max_order_size.parse().unwrap(),
                    min_order_size: self.min_order_size.parse().unwrap(),
                    volume_velocity_threshold: self.volume_velocity.parse().unwrap(),
                    obi_threshold: self.obi_threshold.parse().unwrap(),
                    database_path: "./bot.db".to_string(),
                    rpc_url: None,
                    gemini_api_key: if self.ai_enabled && !self.gemini_api_key.is_empty() {
                        Some(self.gemini_api_key.clone())
                    } else {
                        None
                    },
                    ai_personality: self.ai_personality,
                    ai_enabled: self.ai_enabled,
                    ai_analysis_frequency_minutes: 60,
                });
            }
            _ => {}
        }
        None
    }

    fn validate_trading_params(&self) -> Result<(), String> {
        let max: f64 = self
            .max_order_size
            .parse()
            .map_err(|_| "Max order size inválido".to_string())?;
        let min: f64 = self
            .min_order_size
            .parse()
            .map_err(|_| "Min order size inválido".to_string())?;
        let _vel: f64 = self
            .volume_velocity
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

        if !(0.1..=0.9).contains(&obi) {
            return Err("OBI threshold debe estar entre 0.1 y 0.9".to_string());
        }

        Ok(())
    }

    pub fn get_password(&self) -> &str {
        &self.password
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(area);

        // Title
        let (title, step_num) = match self.step {
            WizardStep::Password => ("🔐 Contraseña Maestra", "1/5"),
            WizardStep::PrivateKey => ("🔑 Private Key", "2/5"),
            WizardStep::GeminiAPI => ("🤖 Gemini API", "3/5"),
            WizardStep::TradingParams => ("⚙️ Parámetros", "4/5"),
            WizardStep::Confirmation => ("✅ Confirmación", "5/5"),
        };

        let title_text = format!("{} - Paso {}", title, step_num);
        let title_widget = Paragraph::new(title_text)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        f.render_widget(title_widget, chunks[0]);

        // Content
        match self.step {
            WizardStep::Password => self.render_password_step(f, chunks[1]),
            WizardStep::PrivateKey => self.render_private_key_step(f, chunks[1]),
            WizardStep::GeminiAPI => self.render_gemini_step(f, chunks[1]),
            WizardStep::TradingParams => self.render_trading_params_step(f, chunks[1]),
            WizardStep::Confirmation => self.render_confirmation_step(f, chunks[1]),
        }

        // Footer
        let footer = match self.step {
            WizardStep::Password => "[ENTER] Continuar  [TAB] Cambiar campo  [ESC] Cancelar",
            WizardStep::PrivateKey | WizardStep::GeminiAPI | WizardStep::TradingParams => {
                "[ENTER] Continuar  [←] Volver  [ESC] Cancelar"
            }
            WizardStep::Confirmation => "[ENTER] Guardar  [←] Volver  [ESC] Cancelar",
        };

        let footer_widget = Paragraph::new(footer)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(footer_widget, chunks[2]);
    }

    fn render_password_step(&self, f: &mut Frame, area: Rect) {
        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Esta contraseña protegerá toda tu configuración.",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(Span::styled(
                "Mínimo 8 caracteres.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw("Ingrese contraseña: "),
                Span::styled(
                    "*".repeat(self.password.len()),
                    Style::default().fg(if self.current_field == 0 {
                        Color::Cyan
                    } else {
                        Color::White
                    }),
                ),
                if self.current_field == 0 {
                    Span::styled("_", Style::default().fg(Color::Cyan))
                } else {
                    Span::raw("")
                },
            ]),
            Line::from(vec![
                Span::raw("Confirme contraseña: "),
                Span::styled(
                    "*".repeat(self.confirm_password.len()),
                    Style::default().fg(if self.current_field == 1 {
                        Color::Cyan
                    } else {
                        Color::White
                    }),
                ),
                if self.current_field == 1 {
                    Span::styled("_", Style::default().fg(Color::Cyan))
                } else {
                    Span::raw("")
                },
            ]),
        ];

        if let Some(ref error) = self.error_message {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                error.clone(),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
        }

        let para = Paragraph::new(lines)
            .block(theme::plain_block(palette::PRIMARY))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });

        f.render_widget(para, area);
    }

    fn render_private_key_step(&self, f: &mut Frame, area: Rect) {
        let masked_key = if self.private_key.len() > 10 {
            format!(
                "{}...{}",
                &self.private_key[..6],
                &self.private_key[self.private_key.len() - 4..]
            )
        } else {
            self.private_key.clone()
        };

        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Tu clave privada para trading en Polymarket.",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(Span::styled(
                "Formato: 0x + 64 caracteres hexadecimales",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw("Private Key: "),
                Span::styled(masked_key, Style::default().fg(Color::Cyan)),
                Span::styled("_", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "⚠️  NUNCA compartas esta clave con nadie",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
        ];

        if let Some(ref error) = self.error_message {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                error.clone(),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
        }

        let para = Paragraph::new(lines)
            .block(theme::plain_block(palette::PRIMARY))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });

        f.render_widget(para, area);
    }

    fn render_gemini_step(&self, f: &mut Frame, area: Rect) {
        let masked_api = if self.gemini_api_key.len() > 10 {
            format!("{}...", &self.gemini_api_key[..10])
        } else {
            self.gemini_api_key.clone()
        };

        let checkbox = if self.ai_enabled { "[✓]" } else { "[ ]" };

        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Para usar el asistente AI, necesitas una API key.",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(Span::styled(
                "Obtén una en: https://aistudio.google.com",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw("API Key: "),
                Span::styled(masked_api, Style::default().fg(Color::Cyan)),
                Span::styled("_", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw(checkbox),
                Span::raw(" Habilitar AI "),
                Span::styled("(presiona ESPACIO)", Style::default().fg(Color::DarkGray)),
            ]),
        ];

        if let Some(ref error) = self.error_message {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                error.clone(),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
        }

        let para = Paragraph::new(lines)
            .block(theme::plain_block(palette::PRIMARY))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });

        f.render_widget(para, area);
    }

    fn render_trading_params_step(&self, f: &mut Frame, area: Rect) {
        let fields = [
            ("Max Order Size (USDC):", &self.max_order_size, "(10-10000)"),
            ("Min Order Size (USDC):", &self.min_order_size, "(0.1-100)"),
            ("Volume Velocity Threshold:", &self.volume_velocity, ""),
            ("OBI Threshold:", &self.obi_threshold, "(0.1-0.9)"),
        ];

        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Configura los parámetros de trading",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(Span::styled(
                "Valores por defecto recomendados",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
        ];

        for (i, (label, value, hint)) in fields.iter().enumerate() {
            let color = if i == self.current_field {
                Color::Cyan
            } else {
                Color::White
            };
            let cursor = if i == self.current_field { "_" } else { "" };

            lines.push(Line::from(vec![
                Span::raw(*label),
                Span::raw(" "),
                Span::styled(value.to_string(), Style::default().fg(color)),
                Span::styled(cursor, Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(*hint, Style::default().fg(Color::DarkGray)),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "[TAB] Siguiente campo",
            Style::default().fg(Color::DarkGray),
        )));

        if let Some(ref error) = self.error_message {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                error.clone(),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
        }

        let para = Paragraph::new(lines)
            .block(theme::plain_block(palette::PRIMARY))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });

        f.render_widget(para, area);
    }

    fn render_confirmation_step(&self, f: &mut Frame, area: Rect) {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Revisa tu configuración:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("✓ ", Style::default().fg(Color::Green)),
                Span::raw("Private Key configurada"),
            ]),
            Line::from(vec![
                Span::styled(
                    if self.ai_enabled { "✓ " } else { "  " },
                    Style::default().fg(Color::Green),
                ),
                Span::raw(if self.ai_enabled {
                    "Gemini API Key configurada"
                } else {
                    "AI deshabilitado"
                }),
            ]),
            Line::from(vec![
                Span::styled("✓ ", Style::default().fg(Color::Green)),
                Span::raw(format!("Max Order: ${}", self.max_order_size)),
            ]),
            Line::from(vec![
                Span::styled("✓ ", Style::default().fg(Color::Green)),
                Span::raw(format!("Min Order: ${}", self.min_order_size)),
            ]),
            Line::from(vec![
                Span::styled("✓ ", Style::default().fg(Color::Green)),
                Span::raw("Trading Thresholds configurados"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "⚠️  Asegúrate de recordar tu contraseña maestra",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
        ];

        let para = Paragraph::new(lines)
            .block(theme::plain_block(palette::POSITIVE))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });

        f.render_widget(para, area);
    }
}

impl Default for ConfigWizard {
    fn default() -> Self {
        Self::new()
    }
}
