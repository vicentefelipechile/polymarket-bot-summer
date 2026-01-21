pub mod cli;
pub mod clob_auth;
pub mod config;
pub mod database;
pub mod execution;
pub mod onboarding;
pub mod spike_detection;
pub mod types;

pub use cli::CLI;
pub use clob_auth::{authenticate, AuthenticatedClient};
pub use config::Config;
pub use database::{init_database, DbPool};
pub use execution::ExecutionEngine;
pub use onboarding::run_onboarding_checks;
pub use spike_detection::SpikeDetector;
