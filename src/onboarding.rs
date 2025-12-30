use colored::*;
use std::env;
use std::fs;
use std::io;

/// Run onboarding checks to ensure the user has all required configuration
pub fn run_onboarding_checks() -> Result<(), OnboardingError> {
    println!("{}", "=".repeat(60).bright_cyan());
    println!("{}", "  Polymarket HFT Bot - Initialization".bright_cyan().bold());
    println!("{}", "=".repeat(60).bright_cyan());
    println!();
    
    // Check 1: Private Key
    check_private_key()?;
    
    // Check 2: API Credentials
    check_api_credentials()?;
    
    // Check 3: Database Permissions
    check_database_permissions()?;
    
    println!("{}", "✓ All configuration checks passed!".green().bold());
    println!();
    
    Ok(())
}

fn check_private_key() -> Result<(), OnboardingError> {
    if env::var("POLYMARKET_PK").is_err() || env::var("POLYMARKET_PK").unwrap().is_empty() {
        return Err(OnboardingError::MissingPrivateKey);
    }
    println!("{} Private key found", "✓".green());
    Ok(())
}

fn check_api_credentials() -> Result<(), OnboardingError> {
    let missing_keys: Vec<&str> = ["CLOB_API_KEY", "CLOB_API_SECRET", "CLOB_PASSPHRASE"]
        .iter()
        .filter(|&&key| env::var(key).is_err() || env::var(key).unwrap().is_empty())
        .copied()
        .collect();
    
    if !missing_keys.is_empty() {
        return Err(OnboardingError::MissingApiCredentials(missing_keys));
    }
    
    println!("{} API credentials found", "✓".green());
    Ok(())
}

fn check_database_permissions() -> Result<(), OnboardingError> {
    let db_path = env::var("DATABASE_PATH").unwrap_or_else(|_| "./bot_history.db".to_string());
    
    // Try to create/open the database file to check permissions
    match fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&db_path)
    {
        Ok(_) => {
            println!("{} Database permissions OK", "✓".green());
            Ok(())
        }
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            Err(OnboardingError::DatabasePermissionDenied(db_path))
        }
        Err(e) => {
            Err(OnboardingError::DatabaseError(e.to_string()))
        }
    }
}

#[derive(Debug)]
pub enum OnboardingError {
    MissingPrivateKey,
    MissingApiCredentials(Vec<&'static str>),
    DatabasePermissionDenied(String),
    DatabaseError(String),
}

impl std::fmt::Display for OnboardingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OnboardingError::MissingPrivateKey => {
                writeln!(f)?;
                writeln!(f, "{}", "[!] CONFIGURATION ERROR: Private Key Not Found".red().bold())?;
                writeln!(f, "{}", "-".repeat(60).red())?;
                writeln!(f, "We could not find your Polygon L1 Private Key.")?;
                writeln!(f)?;
                writeln!(f, "{}", ">> ACTION REQUIRED:".yellow().bold())?;
                writeln!(f, "1. Create a file named {} in the project root folder.", "'.env'".cyan())?;
                writeln!(f, "2. Open your browser wallet (MetaMask/Rabby) connected to Polymarket.")?;
                writeln!(f, "3. Export your Private Key (NOT your Seed Phrase).")?;
                writeln!(f, "4. Add this line to the '.env' file:")?;
                writeln!(f, "   {}", "POLYMARKET_PK=0x...your_key_here...".cyan())?;
                writeln!(f)?;
                writeln!(f, "{}", "TIP: You can copy '.env.example' as a template!".yellow())?;
                writeln!(f, "{}", "-".repeat(60).red())?;
                Ok(())
            }
            OnboardingError::MissingApiCredentials(keys) => {
                writeln!(f)?;
                writeln!(f, "{}", "[!] AUTH ERROR: API Credentials Missing".red().bold())?;
                writeln!(f, "{}", "-".repeat(60).red())?;
                writeln!(f, "We need your CLOB API keys to trade.")?;
                writeln!(f)?;
                writeln!(f, "Missing variables: {}", keys.join(", ").yellow())?;
                writeln!(f)?;
                writeln!(f, "{}", ">> ACTION REQUIRED:".yellow().bold())?;
                writeln!(f, "1. Log in to {}", "https://polymarket.com".cyan())?;
                writeln!(f, "2. Go to {} -> {} -> {}.", 
                    "Settings".cyan(), "API Keys".cyan(), "Create New Key".cyan())?;
                writeln!(f, "3. Copy the 'API Key', 'Secret', and 'Passphrase'.")?;
                writeln!(f, "4. Add them to your '.env' file:")?;
                writeln!(f, "   {}", "CLOB_API_KEY=your_key_here".cyan())?;
                writeln!(f, "   {}", "CLOB_API_SECRET=your_secret_here".cyan())?;
                writeln!(f, "   {}", "CLOB_PASSPHRASE=your_passphrase_here".cyan())?;
                writeln!(f, "{}", "-".repeat(60).red())?;
                Ok(())
            }
            OnboardingError::DatabasePermissionDenied(path) => {
                writeln!(f)?;
                writeln!(f, "{}", "[!] SYSTEM ERROR: Cannot create Database File".red().bold())?;
                writeln!(f, "{}", "-".repeat(60).red())?;
                writeln!(f, "Could not create '{}' in the current folder.", path.yellow())?;
                writeln!(f)?;
                writeln!(f, "{}", ">> DIAGNOSIS:".yellow().bold())?;
                writeln!(f, "- Are you running in a read-only folder?")?;
                writeln!(f, "- Do you have write permissions?")?;
                writeln!(f)?;
                writeln!(f, "{}", ">> TRY:".yellow().bold())?;
                writeln!(f, "- On Linux/Mac: Run {}", "chmod +w .".cyan())?;
                writeln!(f, "- Move to a user directory with write access")?;
                writeln!(f, "- Set DATABASE_PATH in .env to a writable location")?;
                writeln!(f, "{}", "-".repeat(60).red())?;
                Ok(())
            }
            OnboardingError::DatabaseError(err) => {
                writeln!(f)?;
                writeln!(f, "{}", "[!] DATABASE ERROR".red().bold())?;
                writeln!(f, "{}", "-".repeat(60).red())?;
                writeln!(f, "Error: {}", err)?;
                writeln!(f, "{}", "-".repeat(60).red())?;
                Ok(())
            }
        }
    }
}

impl std::error::Error for OnboardingError {}
