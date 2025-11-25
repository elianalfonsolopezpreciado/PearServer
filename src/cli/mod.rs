// Command Line Interface Module
// Powerful CLI using clap for server management

pub mod commands;

use clap::{Parser, Subcommand};
use colored::*;

/// Pear Server - Revolutionary Next-Generation Web Server
#[derive(Parser)]
#[command(name = "pear")]
#[command(author = "Pear Server Team")]
#[command(version = "0.3.0")]
#[command(about = "🍐 Pear Server - WebAssembly-powered web server with self-healing architecture", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the Pear Server daemon
    Start {
        /// Configuration file path
        #[arg(short, long, default_value = "pear.toml")]
        config: String,
        
        /// Run in foreground (don't daemonize)
        #[arg(short, long)]
        foreground: bool,
        
        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Stop the running Pear Server
    Stop {
        /// Force shutdown without graceful period
        #[arg(short, long)]
        force: bool,
    },
    
    /// Show server status
    Status {
        /// Output format: text, json, or table
        #[arg(short, long, default_value = "table")]
        format: String,
    },
    
    /// Deploy a WebAssembly module to a site
    Deploy {
        /// Path to WebAssembly (.wasm) file
        wasm_file: String,
        
        /// Site identifier
        #[arg(short, long, default_value = "default-site")]
        site: String,
        
        /// Number of Cage replicas
        #[arg(short, long, default_value = "3")]
        replicas: usize,
    },
    
    
    /// Set a configuration value
    Set {
        /// Configuration key (e.g., server.http2_port)
        key: String,
        
        /// Configuration value
        value: String,
    },
    
    /// Validate configuration file
    Validate {
        /// Configuration file to validate
        #[arg(short, long, default_value = "pear.toml")]
        file: String,
    },
}

/// Print a success message
pub fn success(msg: &str) {
    println!("{} {}", "✓".green().bold(), msg);
}

/// Print an error message
pub fn error(msg: &str) {
    eprintln!("{} {}", "✗".red().bold(), msg);
}

/// Print an info message
pub fn info(msg: &str) {
    println!("{} {}", "ℹ".blue().bold(), msg);
}

/// Print a warning message
pub fn warning(msg: &str) {
    println!("{} {}", "⚠".yellow().bold(), msg);
}

/// Print the Pear Server banner
pub fn print_banner() {
    println!("{}", r#"
╔═══════════════════════════════════════════════════════════╗
║                                                            ║
║   🍐  PEAR SERVER  v0.3.0                                ║
║                                                            ║
║   Revolutionary WebAssembly-Powered Web Server            ║
║   with Self-Healing Cage Pool Architecture                ║
║                                                            ║
╚═══════════════════════════════════════════════════════════╝
    "#.bright_cyan().bold());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        // Test that CLI can be constructed
        let _cli = Cli::parse_from(&["pear", "start", "--foreground"]);
    }
}
