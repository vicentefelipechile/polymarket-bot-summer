/// Run first-time setup wizard
async fn run_first_time_setup(config_path: &Path) -> Result<SecureConfig> {
    // Setup terminal for wizard
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut wizard = ConfigWizard::new();
    let mut config: Option<SecureConfig> = None;

    // Wizard loop
    while config.is_none() {
        terminal.draw(|f| {
            wizard.render(f, f.area());
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if let Some(completed_config) = wizard.handle_input(key) {
                    config = Some(completed_config);
                }
            }
        }
    }

    cleanup_terminal(&mut terminal)?;

    // Save encrypted configuration with password from wizard
    let config = config.unwrap();
    let password = wizard.get_password();
    config
        .save_to_file(config_path, password)
        .context("Failed to save encrypted configuration")?;

    tracing::info!("✓ Configuración guardada exitosamente");

    Ok(config)
}
