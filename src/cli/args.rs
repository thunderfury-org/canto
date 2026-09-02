use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "canto",
    author = "thunderfury-org",
    version,
    about = "A lightweight transparent proxy and configuration orchestrator for sing-box",
    long_about = None
)]
pub struct Cli {
    /// Custom path to canto configuration file (defaults to canto.toml or /etc/canto/canto.toml)
    #[arg(short, long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Verbose logging level (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run canto in foreground, managing sing-box and transparent proxy rules
    Run(RunArgs),

    /// Start canto background daemon
    Start,

    /// Stop canto service and flush transparent proxy rules
    Stop,

    /// Query the current status of sing-box and network rules
    Status,

    /// Configuration and template management utilities
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Emergency flush of all canto network rules and route policies
    CleanNetwork,
}

#[derive(Args, Debug)]
pub struct RunArgs {
    /// Skip applying network routing and firewall rules (run in pure proxy mode)
    #[arg(long)]
    pub no_network: bool,

    /// Force re-generating sing-box config before launch
    #[arg(long)]
    pub recompile_config: bool,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Generate sing-box config.json from template fragments
    Generate {
        /// Optional path to templates directory
        #[arg(short, long)]
        template_dir: Option<PathBuf>,

        /// Output path for the merged config.json
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Validate the current sing-box configuration syntax using `sing-box check`
    Check {
        /// Path to config.json to check
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Initialize a default canto.toml in the current directory or specified path
    Init {
        /// Target path to write default canto.toml
        #[arg(short, long, default_value = "canto.toml")]
        path: PathBuf,
    },
}
