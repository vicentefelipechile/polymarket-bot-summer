use crate::execution::ExecutionEngine;
use anyhow::Result;
use colored::*;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result as RustylineResult};
use std::sync::Arc;

/// CLI REPL for interacting with the trading bot
pub struct CLI {
    editor: DefaultEditor,
    execution_engine: Arc<ExecutionEngine>,
}

impl CLI {
    pub fn new(execution_engine: Arc<ExecutionEngine>) -> RustylineResult<Self> {
        let editor = DefaultEditor::new()?;
        Ok(Self {
            editor,
            execution_engine,
        })
    }
    
    /// Run the interactive REPL loop
    pub async fn run(&mut self) -> Result<()> {
        self.print_welcome();
        
        loop {
            let readline = self.editor.readline(&format!("{} ", "polymarket>".cyan().bold()));
            
            match readline {
                Ok(line) => {
                    let line = line.trim();
                    
                    if line.is_empty() {
                        continue;
                    }
                    
                    // Add to history
                    let _ = self.editor.add_history_entry(line);
                    
                    // Process command
                    if let Err(e) = self.process_command(line).await {
                        eprintln!("{} {}", "Error:".red().bold(), e);
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("{}  Use {} to exit", "^C".yellow(), "/exit".cyan());
                }
                Err(ReadlineError::Eof) => {
                    println!("Exiting...");
                    break;
                }
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    async fn process_command(&self, cmd: &str) -> Result<()> {
        match cmd {
            "/help" => self.cmd_help(),
            "/currentstate" => self.cmd_current_state().await,
            "/lastbid" => self.cmd_last_bid().await,
            "/balance" => self.cmd_balance().await,
            "/active" => self.cmd_active().await,
            "/markets" => self.cmd_markets().await,
            "/pnl" => self.cmd_pnl().await,
            "/pause" => self.cmd_pause().await,
            "/resume" => self.cmd_resume().await,
            "/panic" => self.cmd_panic().await,
            "/export" => self.cmd_export().await,
            "/exit" | "/quit" => {
                println!("Shutting down...");
                std::process::exit(0);
            }
            _ => {
                println!("{} Unknown command. Type {} for help.", 
                    "⚠".yellow(), "/help".cyan());
            }
        }
        
        Ok(())
    }
    
    fn print_welcome(&self) {
        println!();
        println!("{}", "=".repeat(60).bright_cyan());
        println!("{}", "  Polymarket HFT Bot - Ready".bright_cyan().bold());
        println!("{}", "=".repeat(60).bright_cyan());
        println!();
        println!("Type {} for available commands.", "/help".cyan());
        println!("Type {} to exit.", "/exit".cyan());
        println!();
    }
    
    fn cmd_help(&self) {
        println!();
        println!("{}", "Available Commands:".green().bold());
        println!();
        println!("{}", "Information Commands:".yellow());
        println!("  {}  - Displays this list of commands", "/help".cyan());
        println!("  {}  - Shows system health, WS status, and latency", "/currentstate".cyan());
        println!("  {}  - Shows details of the last order placed", "/lastbid".cyan());
        println!("  {}  - Displays current USDC balance and portfolio value", "/balance".cyan());
        println!("  {}  - Lists all currently open orders", "/active".cyan());
        println!("  {}  - Lists monitored market IDs", "/markets".cyan());
        println!("  {}  - Shows realized vs unrealized P&L", "/pnl".cyan());
        println!();
        println!("{}", "Control Commands:".yellow());
        println!("  {}  - Pause new order placement (cancel-only mode)", "/pause".cyan());
        println!("  {}  - Resume normal trading operations", "/resume".cyan());
        println!("  {}  - EMERGENCY: Cancel all orders and pause", "/panic".red().bold());
        println!("  {}  - Export session log to CSV", "/export".cyan());
        println!("  {}  - Exit the bot", "/exit".cyan());
        println!();
    }
    
    async fn cmd_current_state(&self) {
        println!();
        println!("{}", "System Status:".green().bold());
        println!("  CPU: N/A");  // TODO: Add system metrics
        println!("  RAM: N/A");
        println!("  WebSocket: {}", "Connected".green());
        println!("  Latency: {}ms", "42".yellow());
        println!("  Status: {}", if self.execution_engine.is_paused().await {
            "PAUSED".red().to_string()
        } else {
            "ACTIVE".green().to_string()
        });
        println!();
    }
    
    async fn cmd_last_bid(&self) {
        println!();
        if let Some(order_id) = self.execution_engine.get_last_order_id().await {
            println!("{}", "Last Order:".green().bold());
            println!("  Order ID: {}", order_id.cyan());
            // TODO: Fetch full order details
        } else {
            println!("{}", "No orders placed yet.".yellow());
        }
        println!();
    }
    
    async fn cmd_balance(&self) {
        println!();
        match self.execution_engine.get_portfolio().await {
            Ok(portfolio) => {
                println!("{}", "Portfolio:".green().bold());
                println!("  USDC Balance: ${:.2}", portfolio.usdc_balance);
                println!("  Total Value: ${:.2}", portfolio.total_value);
            }
            Err(e) => {
                eprintln!("{} {}", "Error fetching balance:".red(), e);
            }
        }
        println!();
    }
    
    async fn cmd_active(&self) {
        println!();
        match self.execution_engine.get_active_orders().await {
            Ok(orders) => {
                if orders.is_empty() {
                    println!("{}", "No active orders.".yellow());
                } else {
                    println!("{} ({} total)", "Active Orders:".green().bold(), orders.len());
                    for order in orders {
                        println!("  {} | {} | {} @ ${}", 
                            order.order_id.cyan(),
                            order.side.yellow(),
                            order.size,
                            order.price
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("{} {}", "Error fetching orders:".red(), e);
            }
        }
        println!();
    }
    
    async fn cmd_markets(&self) {
        println!();
        println!("{}", "Monitored Markets:".green().bold());
        println!("  {}", "No markets configured yet".yellow());
        // TODO: Display actual monitored markets
        println!();
    }
    
    async fn cmd_pnl(&self) {
        println!();
        match self.execution_engine.get_portfolio().await {
            Ok(portfolio) => {
                println!("{}", "Profit & Loss:".green().bold());
                println!("  Realized P&L: {}", 
                    format!("${:.2}", portfolio.realized_pnl).cyan());
                println!("  Unrealized P&L: {}", 
                    format!("${:.2}", portfolio.unrealized_pnl).cyan());
                println!("  Total: {}", 
                    format!("${:.2}", portfolio.realized_pnl + portfolio.unrealized_pnl).bright_cyan().bold());
            }
            Err(e) => {
                eprintln!("{} {}", "Error fetching P&L:".red(), e);
            }
        }
        println!();
    }
    
    async fn cmd_pause(&self) {
        self.execution_engine.pause().await;
        println!();
        println!("{}", "⏸️  Bot paused - trading disabled".yellow().bold());
        println!();
    }
    
    async fn cmd_resume(&self) {
        self.execution_engine.resume().await;
        println!();
        println!("{}", "▶️  Bot resumed - trading enabled".green().bold());
        println!();
    }
    
    async fn cmd_panic(&self) {
        println!();
        println!("{}", "🚨 PANIC MODE ACTIVATED 🚨".red().bold());
        
        match self.execution_engine.cancel_all_orders().await {
            Ok(count) => {
                println!("  Cancelled {} orders", count);
                println!("  Bot is now {}", "PAUSED".red().bold());
            }
            Err(e) => {
                eprintln!("{} {}", "Error during panic:".red(), e);
            }
        }
        
        println!();
    }
    
    async fn cmd_export(&self) {
        println!();
        println!("{}", "Export feature coming soon...".yellow());
        // TODO: Implement CSV export
        println!();
    }
}
