pub mod clob_auth;
pub mod config;
pub mod database;
pub mod execution;
pub mod markets;
pub mod onboarding;
pub mod spike_detection;
pub mod tui;
pub mod types;

pub use clob_auth::{authenticate, AuthenticatedClient};
pub use config::Config;
pub use database::{init_database, DbPool};
pub use execution::ExecutionEngine;
pub use onboarding::run_onboarding_checks;
pub use spike_detection::SpikeDetector;
pub use tui::run_tui;
